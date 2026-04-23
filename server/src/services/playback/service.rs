use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
    Snowflake,
    services::{
        playback::{CollapsedResyncState, ResyncState},
        snowflake::SnowflakeService,
    },
};

pub struct PlaybackService {
    resync_states: RwLock<Vec<ResyncState>>,
    snowflake_service: Arc<SnowflakeService>,
}

impl PlaybackService {
    pub fn new(snowflake_service: Arc<SnowflakeService>) -> Self {
        Self {
            resync_states: RwLock::new(Vec::new()),
            snowflake_service,
        }
    }

    /// Initiates a Resync operation
    ///
    /// Returns the Snowflake of the newly created ResyncState
    pub async fn initiate_resync(&self, users: Vec<Snowflake>) -> Snowflake {
        let mut resync_states = self.resync_states.write().await;
        let state_id = self.snowflake_service.generate();
        resync_states.push(ResyncState::new(state_id, users));
        state_id
    }

    /// Updates an existing ResyncState
    ///
    /// Returns whether this operation resulted in a complete state
    pub async fn resync_report(
        &self,
        resync_id: Snowflake,
        user_id: Snowflake,
        timestamp: u32,
    ) -> Result<bool, crate::Error> {
        let mut resync_states = self.resync_states.write().await;
        let state = resync_states
            .iter_mut()
            .find(|s| s.id == resync_id)
            .ok_or(crate::Error::InvalidResyncState(resync_id))?;
        Ok(state.add_timestamp(user_id, timestamp).await)
    }

    pub async fn collapse_resync(
        &self,
        resync_id: Snowflake,
    ) -> Result<CollapsedResyncState, crate::Error> {
        let mut resync_states = self.resync_states.write().await;
        let index = resync_states
            .iter()
            .position(|s| s.id == resync_id)
            .ok_or(crate::Error::InvalidResyncState(resync_id))?;
        let state = resync_states.swap_remove(index);

        let timestamp = state
            .get_min_timestamp()
            .await
            .ok_or(crate::Error::NoResyncTimestamps)?;

        Ok(CollapsedResyncState {
            id: state.id,
            timestamp,
        })
    }
}
