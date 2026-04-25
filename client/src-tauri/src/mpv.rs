use crate::CommandResult;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::Duration;

#[cfg(not(target_os = "windows"))]
use tokio::net::UnixStream;

#[cfg(target_os = "windows")]
use tokio::net::windows::named_pipe::NamedPipeClient;

#[cfg(not(target_os = "windows"))]
type IpcStream = UnixStream;

#[cfg(target_os = "windows")]
type IpcStream = NamedPipeClient;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const EVENT_CHANNEL_CAPACITY: usize = 128;

/// Events surfaced from mpv. Anything not explicitly enumerated arrives as
/// `Other(name)` so unknown events don't silently vanish.
#[derive(Debug, Clone)]
pub enum Event {
    PropertyChange { name: String, data: Value },
    Seek,
    PlaybackRestart,
    FileLoaded,
    EndFile,
    Shutdown,
    Other(String),
}

/// State that only exists while we have a live IPC connection. Dropping this
/// (by setting `MpvManager.state` to `None`) aborts the reader task, drops
/// the writer half, and causes any in-flight `send_command` calls to fail.
struct ConnectedState {
    writer: Arc<Mutex<WriteHalf<IpcStream>>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    events_tx: broadcast::Sender<Event>,
    next_id: Arc<AtomicU64>,
    reader_handle: JoinHandle<()>,
}

impl Drop for ConnectedState {
    fn drop(&mut self) {
        // JoinHandle drop alone detaches the task; we need to explicitly abort.
        self.reader_handle.abort();
    }
}

pub struct MpvManager {
    process: Arc<Mutex<Option<Child>>>,
    socket_path: String,
    state: Arc<Mutex<Option<ConnectedState>>>,
}

impl MpvManager {
    pub fn new() -> Self {
        Self::with_socket_name("mpv-socket")
    }

    pub fn with_socket_name(name: &str) -> Self {
        #[cfg(not(target_os = "windows"))]
        let socket_path = std::env::temp_dir()
            .join(name)
            .to_string_lossy()
            .into_owned();

        #[cfg(target_os = "windows")]
        let socket_path = format!(r"\\.\pipe\{}", name);

        Self {
            process: Arc::new(Mutex::new(None)),
            socket_path,
            state: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&self, mpv_path: &str, video_path: &str) -> CommandResult<()> {
        self.stop().await?;

        let child = Command::new(mpv_path)
            .arg(format!("--input-ipc-server={}", self.socket_path))
            .arg("--idle=yes")
            .arg("--pause")
            .arg("--force-window=yes")
            .arg(video_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start mpv: {}", e))?;

        *self.process.lock().await = Some(child);

        self.connect_with_retry(10, 100).await?;
        Ok(())
    }

    async fn connect_with_retry(&self, max_attempts: u32, delay_ms: u64) -> CommandResult<()> {
        let mut last_error = String::new();
        for _ in 0..max_attempts {
            match self.connect_once().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_error = e;
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
        Err(format!(
            "Failed to connect after {} attempts: {}",
            max_attempts, last_error
        ))
    }

    async fn connect_once(&self) -> CommandResult<()> {
        let stream = open_stream(&self.socket_path).await?;
        let (reader, writer) = tokio::io::split(stream);

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (events_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);

        let reader_handle = tokio::spawn(reader_task(reader, pending.clone(), events_tx.clone()));

        *self.state.lock().await = Some(ConnectedState {
            writer: Arc::new(Mutex::new(writer)),
            pending,
            events_tx,
            next_id: Arc::new(AtomicU64::new(1)),
            reader_handle,
        });

        Ok(())
    }

    pub async fn stop(&self) -> CommandResult<()> {
        // Best-effort graceful shutdown: tell mpv to quit if we're connected.
        // The `let` binding is important: we must drop the MutexGuard before
        // calling `send_command`, which also needs to lock `state`.
        let connected = self.state.lock().await.is_some();
        if connected {
            let _ = self.send_command(vec![json!("quit")]).await;
        }

        // Dropping ConnectedState aborts the reader, closes the writer, and
        // fails any in-flight pending responses with "channel dropped".
        *self.state.lock().await = None;

        if let Some(mut process) = self.process.lock().await.take() {
            let _ = process.kill().await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::fs::remove_file(&self.socket_path);
        }

        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        self.process.lock().await.is_some()
    }

    /// Subscribe to mpv events. Returns `Err` if not currently connected.
    /// Multiple subscribers are allowed; each gets its own receiver.
    pub async fn subscribe_events(&self) -> CommandResult<broadcast::Receiver<Event>> {
        let state = self.state.lock().await;
        let state = state
            .as_ref()
            .ok_or_else(|| "Not connected to mpv".to_string())?;
        Ok(state.events_tx.subscribe())
    }

    /// Send a command and await mpv's JSON response. Every mpv command produces
    /// a response; for commands that carry no data (like `set_property`) the
    /// returned `Value` is `Value::Null`.
    async fn send_command(&self, command: Vec<Value>) -> CommandResult<Value> {
        // Grab the handles we need while briefly holding the state lock.
        let (writer, pending, req_id) = {
            let state = self.state.lock().await;
            let state = state
                .as_ref()
                .ok_or_else(|| "Not connected to mpv".to_string())?;
            let req_id = state.next_id.fetch_add(1, Ordering::Relaxed);
            (state.writer.clone(), state.pending.clone(), req_id)
        };

        let (tx, rx) = oneshot::channel();
        pending.lock().await.insert(req_id, tx);

        let payload = json!({ "command": command, "request_id": req_id });
        let cmd_str = format!("{}\n", payload);

        // Write the command. On failure, remove our entry so it doesn't leak.
        {
            let mut w = writer.lock().await;
            if let Err(e) = w.write_all(cmd_str.as_bytes()).await {
                pending.lock().await.remove(&req_id);
                return Err(format!("Failed to send command: {}", e));
            }
        }

        match tokio::time::timeout(COMMAND_TIMEOUT, rx).await {
            Ok(Ok(Ok(value))) => Ok(value),
            Ok(Ok(Err(e))) => Err(format!("mpv error: {}", e)),
            Ok(Err(_)) => Err("Response channel dropped (connection closed?)".to_string()),
            Err(_) => {
                // Timed out: clean up so the sender doesn't leak.
                pending.lock().await.remove(&req_id);
                Err("Timed out waiting for mpv response".to_string())
            }
        }
    }

    // --- Playback control ---

    pub async fn pause(&self) -> CommandResult<()> {
        self.set_property("pause", json!(true)).await
    }

    pub async fn play(&self) -> CommandResult<()> {
        self.set_property("pause", json!(false)).await
    }

    pub async fn seek_absolute(&self, seconds: f64) -> CommandResult<()> {
        self.send_command(vec![json!("seek"), json!(seconds), json!("absolute")])
            .await
            .map(|_| ())
    }

    pub async fn seek_relative(&self, delta_seconds: f64) -> CommandResult<()> {
        self.send_command(vec![json!("seek"), json!(delta_seconds), json!("relative")])
            .await
            .map(|_| ())
    }

    /// Current playback position in seconds. Fails if mpv has no file loaded
    /// (time-pos will be null in that case).
    pub async fn get_time_pos(&self) -> CommandResult<f64> {
        let v = self.get_property("time-pos").await?;
        v.as_f64()
            .ok_or_else(|| format!("time-pos not a number: {}", v))
    }

    // --- Property / observation primitives ---

    pub async fn get_property(&self, name: &str) -> CommandResult<Value> {
        self.send_command(vec![json!("get_property"), json!(name)])
            .await
    }

    pub async fn set_property(&self, name: &str, value: Value) -> CommandResult<()> {
        self.send_command(vec![json!("set_property"), json!(name), value])
            .await
            .map(|_| ())
    }

    /// Ask mpv to send `Event::PropertyChange` whenever `name`'s value changes.
    /// `id` is an arbitrary identifier you choose; pass it to `unobserve_property`
    /// to cancel. mpv sends one change event immediately on subscription.
    pub async fn observe_property(&self, id: u64, name: &str) -> CommandResult<()> {
        self.send_command(vec![json!("observe_property"), json!(id), json!(name)])
            .await
            .map(|_| ())
    }

    pub async fn unobserve_property(&self, id: u64) -> CommandResult<()> {
        self.send_command(vec![json!("unobserve_property"), json!(id)])
            .await
            .map(|_| ())
    }

    // --- Tracks ---

    pub async fn set_subtitle_track(&self, index: u32) -> CommandResult<()> {
        self.set_property("sid", json!(index)).await
    }

    pub async fn set_audio_track(&self, index: u32) -> CommandResult<()> {
        self.set_property("aid", json!(index)).await
    }
}

// --- Internals ---

#[cfg(not(target_os = "windows"))]
async fn open_stream(path: &str) -> CommandResult<IpcStream> {
    UnixStream::connect(path)
        .await
        .map_err(|e| format!("Failed to connect to mpv socket: {}", e))
}

#[cfg(target_os = "windows")]
async fn open_stream(path: &str) -> CommandResult<IpcStream> {
    use tokio::net::windows::named_pipe::ClientOptions;
    ClientOptions::new()
        .open(path)
        .map_err(|e| format!("Failed to connect to mpv pipe: {}", e))
}

async fn reader_task(
    reader: ReadHalf<IpcStream>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    events_tx: broadcast::Sender<Event>,
) {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF: mpv closed the socket
            Ok(_) => {}
            Err(_) => break, // read error, treat as disconnect
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue, // skip malformed line, keep reading
        };

        // Responses carry a request_id; events don't.
        if let Some(req_id) = msg.get("request_id").and_then(|v| v.as_u64()) {
            let result = if msg.get("error").and_then(|e| e.as_str()) == Some("success") {
                Ok(msg.get("data").cloned().unwrap_or(Value::Null))
            } else {
                let err = msg
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                Err(err)
            };

            if let Some(sender) = pending.lock().await.remove(&req_id) {
                let _ = sender.send(result);
            }
        } else if let Some(event_name) = msg.get("event").and_then(|v| v.as_str()) {
            let event = match event_name {
                "property-change" => Event::PropertyChange {
                    name: msg
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    data: msg.get("data").cloned().unwrap_or(Value::Null),
                },
                "seek" => Event::Seek,
                "playback-restart" => Event::PlaybackRestart,
                "file-loaded" => Event::FileLoaded,
                "end-file" => Event::EndFile,
                "shutdown" => Event::Shutdown,
                other => Event::Other(other.to_string()),
            };
            // No subscribers is fine, so ignore send errors.
            let _ = events_tx.send(event);
        }
        // else: unknown message shape, silently skip
    }
}
