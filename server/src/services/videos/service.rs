use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
    Snowflake,
    services::{
        snowflake::SnowflakeService,
        videos::{Index, VIDEO_EXTENSIONS, VideoEntry, parse_snowflake_stem},
    },
};

pub struct VideoService {
    dir: PathBuf,
    index: RwLock<Index>,
    snowflake_service: Arc<SnowflakeService>,
}

impl VideoService {
    pub async fn new(
        videos_dir: impl Into<PathBuf>,
        snowflake_service: Arc<SnowflakeService>,
    ) -> Result<Self, crate::Error> {
        // Permissive validator: accept every video file. If the filename
        // already stems to a snowflake, preserve it (no rename needed).
        // Otherwise mint a fresh id; `load` will rename the file on disk.
        let videos_dir = videos_dir.into();
        let sf = snowflake_service.clone();
        let index = Index::load(&videos_dir, move |path| {
            Some(parse_snowflake_stem(path).unwrap_or_else(|| sf.generate()))
        })
        .await
        .map_err(|e| crate::Error::Internal(e.to_string()))?;

        Ok(Self {
            dir: videos_dir,
            index: RwLock::new(index),
            snowflake_service,
        })
    }

    pub async fn get_playlist(&self) -> Vec<VideoEntry> {
        let index = self.index.read().await;
        sorted_playlist(&index)
    }

    pub async fn get_entry(&self, id: Snowflake) -> Option<VideoEntry> {
        self.index.read().await.get(id).cloned()
    }

    pub async fn set_title(
        &self,
        id: Snowflake,
        title: String,
    ) -> Result<VideoEntry, crate::Error> {
        let mut index = self.index.write().await;
        let entry = index.get_mut(id).ok_or(crate::Error::InvalidVideo(id))?;
        entry.display_name = title;
        let updated = entry.clone();
        index
            .save()
            .await
            .map_err(|e| crate::Error::Internal(e.to_string()))?;
        Ok(updated)
    }

    pub async fn set_audio_track(
        &self,
        id: Snowflake,
        track_idx: i32,
    ) -> Result<VideoEntry, crate::Error> {
        let mut index = self.index.write().await;
        let entry = index.get_mut(id).ok_or(crate::Error::InvalidVideo(id))?;
        entry.audio_track = track_idx;
        let updated = entry.clone();
        index
            .save()
            .await
            .map_err(|e| crate::Error::Internal(e.to_string()))?;
        Ok(updated)
    }

    pub async fn set_subtitle_track(
        &self,
        id: Snowflake,
        track_idx: i32,
    ) -> Result<VideoEntry, crate::Error> {
        let mut index = self.index.write().await;
        let entry = index.get_mut(id).ok_or(crate::Error::InvalidVideo(id))?;
        entry.subtitle_track = track_idx;
        let updated = entry.clone();
        index
            .save()
            .await
            .map_err(|e| crate::Error::Internal(e.to_string()))?;
        Ok(updated)
    }

    /// Swap the order fields of two entries. Note: this is a two-item swap,
    /// not a general reorder — it can't express arbitrary drag-and-drop moves.
    pub async fn reorder_entries(
        &self,
        left: Snowflake,
        right: Snowflake,
    ) -> Result<Vec<VideoEntry>, crate::Error> {
        let mut index = self.index.write().await;

        let order_l = index
            .get(left)
            .ok_or(crate::Error::InvalidVideo(left))?
            .order;
        let order_r = index
            .get(right)
            .ok_or(crate::Error::InvalidVideo(right))?
            .order;

        index.get_mut(left).expect("validated above").order = order_r;
        index.get_mut(right).expect("validated above").order = order_l;

        index
            .save()
            .await
            .map_err(|e| crate::Error::Internal(e.to_string()))?;
        Ok(sorted_playlist(&index))
    }

    /// Generate an id and compute the final on-disk path. Does not touch
    /// the filesystem or index — call `register` once the file is in place.
    pub fn reserve(&self, ext: &str) -> (Snowflake, PathBuf) {
        let id = self.snowflake_service.generate();
        let path = self.dir.join(format!("{id}.{ext}"));
        (id, path)
    }

    /// Register a file that's already been written to `{id}.{ext}` and
    /// persist the index.
    pub async fn register(&self, id: Snowflake, display_name: String) -> std::io::Result<()> {
        let mut index = self.index.write().await;
        index.insert(
            VideoEntry {
                id,
                display_name,
                audio_track: 0,
                subtitle_track: 0,
                order: 0,
            },
            None,
        );
        index.save().await
    }

    /// Resolve the on-disk path by trying each known extension. `None` if
    /// the entry isn't in the index or no matching file exists.
    pub async fn resolve_path(&self, id: Snowflake) -> Option<PathBuf> {
        self.index.read().await.get(id)?;
        for ext in VIDEO_EXTENSIONS {
            let candidate = self.dir.join(format!("{id}.{ext}"));
            if tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
                return Some(candidate);
            }
        }
        None
    }

    pub async fn display_name(&self, id: Snowflake) -> Option<String> {
        self.index
            .read()
            .await
            .get(id)
            .map(|e| e.display_name.clone())
    }

    pub async fn list_ids(&self) -> Vec<Snowflake> {
        let index = self.index.read().await;
        index.iter().map(|e| e.id).collect()
    }
}

fn sorted_playlist(index: &Index) -> Vec<VideoEntry> {
    let mut playlist: Vec<VideoEntry> = index.iter().cloned().collect();
    playlist.sort_by_key(|e| e.order);
    playlist
}
