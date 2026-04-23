use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult},
    core::AppState,
    services::permissions::Permissions,
    websocket::ServerMessage,
};

pub struct SelectVideoCommand {
    pub video_id: Snowflake,
}

#[async_trait]
impl Command for SelectVideoCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        _user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        if state
            .services()
            .video()
            .resolve_path(self.video_id)
            .await
            .is_none()
        {
            return Err(crate::Error::InvalidVideo(self.video_id));
        }
        state.services().playback().select(self.video_id).await;
        Ok(ServerMessage::VideoSelected {
            video_id: self.video_id,
        }
        .into())
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_PLAYBACK)
    }
}
