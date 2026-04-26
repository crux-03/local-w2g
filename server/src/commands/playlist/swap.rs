use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult},
    core::AppState,
    services::permissions::Permissions,
    websocket::ServerMessage,
};

pub struct SwapEntriesCommand {
    pub first: Snowflake,
    pub second: Snowflake,
}

#[async_trait]
impl Command for SwapEntriesCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        _user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let playlist = state
            .services()
            .video()
            .reorder_entries(self.first, self.second)
            .await?;

        Ok(ServerMessage::PlaylistUpdated { playlist }.into())
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_MEDIA)
    }
}
