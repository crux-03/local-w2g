use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
    Snowflake,
    services::{
        videos::{Index, VideoEntry, parse_snowflake_stem},
        snowflake::SnowflakeService,
    },
};

pub struct VideoService {
    index: RwLock<Index>,
    snowflake_service: Arc<SnowflakeService>,
}

impl VideoService {
    pub fn new(
        videos_dir: impl Into<PathBuf>,
        snowflake_service: Arc<SnowflakeService>,
    ) -> Result<Self, crate::Error> {
        // Permissive validator: accept every video file. If the filename
        // already stems to a snowflake, preserve it (no rename needed).
        // Otherwise mint a fresh id; `load` will rename the file on disk.
        let sf = snowflake_service.clone();
        let index = Index::load(videos_dir, move |path| {
            Some(parse_snowflake_stem(path).unwrap_or_else(|| sf.generate().into()))
        })
        .map_err(|e| crate::Error::Internal(e.to_string()))?;

        Ok(Self {
            index: RwLock::new(index),
            snowflake_service,
        })
    }

    pub async fn get_playlist(&self) -> Vec<VideoEntry> {
        let index = self.index.read().await;
        sorted_playlist(&index)
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
            .map_err(|e| crate::Error::Internal(e.to_string()))?;
        Ok(updated)
    }

    pub async fn set_audio_track(
        &self,
        id: Snowflake,
        track_idx: usize,
    ) -> Result<VideoEntry, crate::Error> {
        let mut index = self.index.write().await;
        let entry = index.get_mut(id).ok_or(crate::Error::InvalidVideo(id))?;
        entry.audio_track = track_idx;
        let updated = entry.clone();
        index
            .save()
            .map_err(|e| crate::Error::Internal(e.to_string()))?;
        Ok(updated)
    }

    pub async fn set_subtitle_track(
        &self,
        id: Snowflake,
        track_idx: usize,
    ) -> Result<VideoEntry, crate::Error> {
        let mut index = self.index.write().await;
        let entry = index.get_mut(id).ok_or(crate::Error::InvalidVideo(id))?;
        entry.subtitle_track = track_idx;
        let updated = entry.clone();
        index
            .save()
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
            .map_err(|e| crate::Error::Internal(e.to_string()))?;
        Ok(sorted_playlist(&index))
    }
}

fn sorted_playlist(index: &Index) -> Vec<VideoEntry> {
    let mut playlist: Vec<VideoEntry> = index.iter().cloned().collect();
    playlist.sort_by_key(|e| e.order);
    playlist
}
