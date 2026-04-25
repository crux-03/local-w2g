//! Local file manager for downloaded videos.
//!
//! Owns a single directory of `{snowflake}.{ext}` files plus transient
//! `.partial` files. Performs HTTP downloads, tracks which video IDs are
//! on device or actively downloading, and emits a stream of `FileEvent`s.
//!
//! The manager makes no decisions about *when* to download; that's the
//! caller's job. Translation from `FileEvent` to WS wire messages also
//! happens elsewhere — the manager's only protocol-specific knowledge is
//! that the download response carries an `X-Widget-Id` header which it
//! surfaces via `FileEvent::Started`.
//!
//! `Snowflake` must implement `Copy + Eq + Hash + FromStr + Display`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, Mutex, Notify};

// Adjust to your project's import path.
use crate::protocol::Snowflake;

const EVENT_CHANNEL_CAPACITY: usize = 256;
const PROGRESS_THROTTLE: Duration = Duration::from_millis(250);
const PROGRESS_THROTTLE_PCT: u32 = 1;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum FileEvent {
    /// Emitted once per download, immediately after the HTTP response
    /// arrives and the `X-Widget-Id` header is parsed. Provides the
    /// `video_id → widget_id` mapping that downstream consumers need to
    /// route progress updates to the correct chat widget.
    Started {
        video_id: Snowflake,
        widget_id: Snowflake,
    },
    Progress {
        video_id: Snowflake,
        bytes_done: u64,
        bytes_total: u64,
    },
    Completed {
        video_id: Snowflake,
    },
    Failed {
        video_id: Snowflake,
        reason: String,
    },
    Removed {
        video_id: Snowflake,
    },
}

pub struct FileManager {
    dir: PathBuf,
    state: Mutex<State>,
    events: broadcast::Sender<FileEvent>,
    http: reqwest::Client,
}

struct State {
    on_device: HashMap<Snowflake, PathBuf>,
    /// One `Notify` per active download. Notifying it cancels the download.
    /// The download task is responsible for removing its own entry on exit.
    active: HashMap<Snowflake, Arc<Notify>>,
}

impl FileManager {
    /// Construct a manager rooted at `dir`. Scans the directory:
    /// - any `*.partial` files are deleted (no resume support)
    /// - any `{snowflake}.{ext}` files are recorded as on-device
    /// - other files are ignored
    ///
    /// Returns an `Arc<Self>` because `start_download` needs to clone the
    /// manager into the spawned task; making this the only entry point
    /// means callers can't accidentally hold a non-Arc reference.
    pub fn new(dir: PathBuf) -> std::io::Result<Arc<Self>> {
        std::fs::create_dir_all(&dir)?;
        let mut on_device = HashMap::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) == Some("partial") {
                let _ = std::fs::remove_file(&path);
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str());
            if let Some(s) = stem {
                if let Ok(id) = s.parse::<Snowflake>() {
                    on_device.insert(id, path);
                }
            }
        }
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Ok(Arc::new(Self {
            dir,
            state: Mutex::new(State {
                on_device,
                active: HashMap::new(),
            }),
            events,
            http: reqwest::Client::new(),
        }))
    }

    pub async fn is_on_device(&self, id: Snowflake) -> bool {
        self.state.lock().await.on_device.contains_key(&id)
    }

    pub async fn local_path(&self, id: Snowflake) -> Option<PathBuf> {
        self.state.lock().await.on_device.get(&id).cloned()
    }

    pub async fn on_device_set(&self) -> Vec<Snowflake> {
        self.state.lock().await.on_device.keys().copied().collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<FileEvent> {
        self.events.subscribe()
    }

    /// Begin downloading `id` from `url` into `{id}.{extension}`. No-op if
    /// already on device or already downloading. Outcome is observed via
    /// the event stream: `Progress*` → `Completed`, or `Failed`, or silence
    /// (in the cancellation case).
    pub async fn start_download(
        self: &Arc<Self>,
        id: Snowflake,
        url: reqwest::Url,
        extension: &str,
    ) {
        let cancel = {
            let mut state = self.state.lock().await;
            if state.on_device.contains_key(&id) || state.active.contains_key(&id) {
                return;
            }
            let cancel = Arc::new(Notify::new());
            state.active.insert(id, Arc::clone(&cancel));
            cancel
        };

        let final_path = self.dir.join(format!("{}.{}", id, extension));
        let partial_path = self.dir.join(format!("{}.{}.partial", id, extension));
        let me = Arc::clone(self);

        tokio::spawn(async move {
            let result = me
                .run_download(id, url, &final_path, &partial_path, cancel)
                .await;

            let mut state = me.state.lock().await;
            state.active.remove(&id);

            match result {
                Ok(true) => {
                    state.on_device.insert(id, final_path);
                    drop(state);
                    let _ = me.events.send(FileEvent::Completed { video_id: id });
                }
                Ok(false) => {
                    // Cancelled. Partial file already cleaned up. No event by design.
                }
                Err(reason) => {
                    drop(state);
                    let _ = fs::remove_file(&partial_path).await;
                    let _ = me.events.send(FileEvent::Failed {
                        video_id: id,
                        reason,
                    });
                }
            }
        });
    }

    /// Returns `Ok(true)` on completion, `Ok(false)` on cancellation,
    /// `Err(reason)` on failure.
    async fn run_download(
        &self,
        id: Snowflake,
        url: reqwest::Url,
        final_path: &Path,
        partial_path: &Path,
        cancel: Arc<Notify>,
    ) -> Result<bool, String> {
        let mut response = self.http.get(url).send().await.map_err(|e| e.to_string())?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("HTTP {}", status));
        }

        let widget_id: Snowflake = response
            .headers()
            .get("X-Widget-Id")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| "missing or non-ascii X-Widget-Id header".to_string())?
            .parse()
            .map_err(|_| "invalid X-Widget-Id header".to_string())?;

        let _ = self.events.send(FileEvent::Started {
            video_id: id,
            widget_id,
        });

        let total = response.content_length().unwrap_or(0);
        let mut file = fs::File::create(partial_path)
            .await
            .map_err(|e| e.to_string())?;

        let mut bytes_done: u64 = 0;
        let mut last_emit = Instant::now();
        let mut last_pct: u32 = 0;

        // Initial 0% so the UI shows a bar immediately.
        let _ = self.events.send(FileEvent::Progress {
            video_id: id,
            bytes_done: 0,
            bytes_total: total,
        });

        let cancelled = cancel.notified();
        tokio::pin!(cancelled);

        loop {
            tokio::select! {
                _ = &mut cancelled => {
                    drop(file);
                    let _ = fs::remove_file(partial_path).await;
                    return Ok(false);
                }
                result = response.chunk() => {
                    match result.map_err(|e| e.to_string())? {
                        Some(bytes) => {
                            file.write_all(&bytes).await.map_err(|e| e.to_string())?;
                            bytes_done += bytes.len() as u64;
                            let pct = if total > 0 {
                                (bytes_done * 100 / total) as u32
                            } else {
                                0
                            };
                            if last_emit.elapsed() >= PROGRESS_THROTTLE
                                || pct.saturating_sub(last_pct) >= PROGRESS_THROTTLE_PCT
                            {
                                last_emit = Instant::now();
                                last_pct = pct;
                                let _ = self.events.send(FileEvent::Progress {
                                    video_id: id,
                                    bytes_done,
                                    bytes_total: total,
                                });
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        file.flush().await.map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(partial_path, final_path)
            .await
            .map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// Cancel an in-flight download. No-op if no download is in flight for
    /// `id`. The `.partial` file is cleaned up by the download task.
    /// Removal from `active` also happens in the task, so a subsequent
    /// `start_download` for the same id may briefly no-op until the task
    /// finishes its cleanup — acceptable for v1.
    pub async fn cancel_download(&self, id: Snowflake) {
        if let Some(notify) = self.state.lock().await.active.get(&id) {
            notify.notify_one();
        }
    }

    /// Remove an on-device file. No-op if not on device. Does not cancel
    /// an active download — call `cancel_download` for that.
    pub async fn delete(&self, id: Snowflake) {
        let path = {
            let mut state = self.state.lock().await;
            state.on_device.remove(&id)
        };
        if let Some(path) = path {
            let _ = fs::remove_file(&path).await;
            let _ = self.events.send(FileEvent::Removed { video_id: id });
        }
    }
}
