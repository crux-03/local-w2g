use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

mod service;

pub use service::VideoService;

use crate::Snowflake;

/// Video file extensions tracked in the data directory. Anything else
/// (including `.index` itself) is ignored during discovery.
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "webm", "mov"];

/// Carrier filename.
pub const INDEX_FILENAME: &str = ".index";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEntry {
    pub id: Snowflake,
    pub display_name: String,
    pub audio_track: usize,
    pub subtitle_track: usize,
    /// Position in the ordered list, contiguous from 0. Carrier files written
    /// before this field existed deserialize to 0; the load-time sort by
    /// `(order, id)` then renumbers deterministically.
    #[serde(default)]
    pub order: usize,
}

impl VideoEntry {
    /// Default entry for a file discovered on disk without a carrier entry.
    /// The `order` passed here is provisional; load-time compaction assigns
    /// the real value.
    fn from_discovered(id: Snowflake) -> Self {
        Self {
            id,
            display_name: id.to_string(),
            audio_track: 0,
            subtitle_track: 0,
            order: 0,
        }
    }
}

pub struct Index {
    dir: PathBuf,
    entries: HashMap<Snowflake, VideoEntry>,
}

impl Index {
    /// Load the index from `dir`, reconciling the carrier file against the
    /// actual files on disk.
    ///
    /// **This call has filesystem write side effects.** Any video file in
    /// `dir` that isn't already tracked in the carrier is handed to
    /// `validate`. If the callback returns `Some(id)`, the file is renamed
    /// on disk to `{id}.{ext}` (if not already so named) and a default
    /// entry is created. If the callback returns `None`, the file is
    /// ignored.
    ///
    /// Matching rule for "already tracked": the file's stem parses to a
    /// `Snowflake` *and* that id is present in the carrier. Everything else
    /// is passed to `validate`.
    ///
    /// After reconciliation, carrier entries whose file is missing from
    /// disk are dropped unconditionally. Surviving entries are sorted by
    /// `(order, id)` and renumbered to contiguous `0..m`. Newly added
    /// entries are appended at `m..n` in `read_dir` discovery order.
    ///
    /// # Errors
    ///
    /// Returns `AlreadyExists` if `validate` assigns an id that's already
    /// in use (either in the carrier, or assigned earlier in the same
    /// run), or if the target rename path already exists on disk.
    pub async fn load<F>(dir: impl Into<PathBuf>, mut validate: F) -> io::Result<Self>
    where
        F: FnMut(&Path) -> Option<Snowflake>,
    {
        let dir = dir.into();
        let index_path = dir.join(INDEX_FILENAME);

        // Parse carrier (or start empty).
        let mut entries: HashMap<Snowflake, VideoEntry> = match tokio::fs::read(&index_path).await {
            Ok(bytes) => {
                let list: Vec<VideoEntry> = serde_json::from_slice(&bytes)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                list.into_iter().map(|e| (e.id, e)).collect()
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => HashMap::new(),
            Err(e) => return Err(e),
        };

        // Phase 1: scan dir.
        let mut kept_ids: HashSet<Snowflake> = HashSet::new();
        let mut new_ids: Vec<Snowflake> = Vec::new();

        let mut read_dir = tokio::fs::read_dir(&dir).await?;
        while let Some(dir_entry) = read_dir.next_entry().await? {
            if !dir_entry.file_type().await?.is_file() {
                continue;
            }
            let path = dir_entry.path();
            let ext = match video_ext(&path) {
                Some(e) => e,
                None => continue,
            };

            if let Some(id) = parse_snowflake_stem(&path) {
                if entries.contains_key(&id) {
                    kept_ids.insert(id);
                    continue;
                }
            }

            let id = match validate(&path) {
                Some(id) => id,
                None => continue,
            };

            if entries.contains_key(&id) || kept_ids.contains(&id) {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("validate() assigned id {} which is already in use", id),
                ));
            }

            let target = dir.join(format!("{}.{}", id, ext));
            if path != target {
                if tokio::fs::try_exists(&target).await? {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!("cannot rename {:?}: {:?} already exists", path, target),
                    ));
                }
                tokio::fs::rename(&path, &target).await?;
            }

            entries.insert(id, VideoEntry::from_discovered(id));
            kept_ids.insert(id);
            new_ids.push(id);
        }

        // Phase 2: drop carrier entries whose file vanished.
        entries.retain(|id, _| kept_ids.contains(id));

        // Phase 3: compact orders. (Unchanged — no I/O.)
        let new_set: HashSet<Snowflake> = new_ids.iter().copied().collect();

        let mut survivors: Vec<VideoEntry> = Vec::new();
        let mut new_entries: HashMap<Snowflake, VideoEntry> = HashMap::new();
        for (id, e) in entries.into_iter() {
            if new_set.contains(&id) {
                new_entries.insert(id, e);
            } else {
                survivors.push(e);
            }
        }
        survivors.sort_by_key(|e| (e.order, e.id));
        for (i, e) in survivors.iter_mut().enumerate() {
            e.order = i;
        }
        let mut next_order = survivors.len();
        for id in new_ids {
            let mut e = new_entries.remove(&id).expect("id came from new_ids");
            e.order = next_order;
            survivors.push(e);
            next_order += 1;
        }

        let entries: HashMap<Snowflake, VideoEntry> =
            survivors.into_iter().map(|e| (e.id, e)).collect();

        Ok(Self { dir, entries })
    }

    /// Write the current state to `{dir}/.index` atomically via write-then-rename.
    /// Entries are serialized in `order` sequence.
    pub async fn save(&self) -> io::Result<()> {
        let mut list: Vec<&VideoEntry> = self.entries.values().collect();
        list.sort_by_key(|e| (e.order, e.id));

        let json = serde_json::to_vec_pretty(&list)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let final_path = self.dir.join(INDEX_FILENAME);
        let tmp_path = self.dir.join(format!("{}.tmp", INDEX_FILENAME));
        tokio::fs::write(&tmp_path, json).await?;
        tokio::fs::rename(tmp_path, final_path).await?;
        Ok(())
    }

    /// Insert `entry` at position `order`, shifting existing entries as needed
    /// to preserve contiguous `0..n` ordering.
    ///
    /// - `None` or `Some(k)` where `k > entries.len()` → appended at the tail.
    /// - `Some(k)` where `k <= entries.len()` → placed at `k`; entries with
    ///   order `>= k` shift up by one.
    ///
    /// If an entry with the same id already exists, it is removed first
    /// (with its gap healed) and returned. Re-inserting an existing id with
    /// `order = None` therefore moves the entry to the tail — use
    /// [`Index::get_mut`] instead if you want to update metadata in place.
    pub fn insert(&mut self, entry: VideoEntry, order: Option<usize>) -> Option<VideoEntry> {
        let replaced = self.remove(entry.id);

        let n = self.entries.len();
        let target = match order {
            Some(k) if k <= n => k,
            _ => n,
        };

        for e in self.entries.values_mut() {
            if e.order >= target {
                e.order += 1;
            }
        }

        let mut new_entry = entry;
        new_entry.order = target;
        self.entries.insert(new_entry.id, new_entry);

        replaced
    }

    pub fn get(&self, id: Snowflake) -> Option<&VideoEntry> {
        self.entries.get(&id)
    }

    pub fn get_mut(&mut self, id: Snowflake) -> Option<&mut VideoEntry> {
        self.entries.get_mut(&id)
    }

    /// Remove the entry with `id`, returning it if present. Heals the gap
    /// left behind by decrementing the `order` of every subsequent entry,
    /// preserving the contiguous `0..n` invariant.
    pub fn remove(&mut self, id: Snowflake) -> Option<VideoEntry> {
        let removed = self.entries.remove(&id)?;
        let old_order = removed.order;
        for e in self.entries.values_mut() {
            if e.order > old_order {
                e.order -= 1;
            }
        }
        Some(removed)
    }

    pub fn iter(&self) -> impl Iterator<Item = &VideoEntry> {
        self.entries.values()
    }
}

/// Parse the file stem of `path` as a `Snowflake`. Returns `None` if the
/// stem is missing, not UTF-8, or not a valid snowflake id. Extension is
/// not consulted.
pub fn parse_snowflake_stem(path: &Path) -> Option<Snowflake> {
    path.file_stem()?.to_str()?.parse::<Snowflake>().ok()
}

/// Returns the path's extension if and only if it matches one of
/// [`VIDEO_EXTENSIONS`] (case-insensitive). The returned string is the
/// extension as it appears on disk (case preserved).
fn video_ext(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    if VIDEO_EXTENSIONS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(ext))
    {
        Some(ext.to_string())
    } else {
        None
    }
}
