use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult, Effect},
    core::AppState,
    websocket::ServerMessage,
};

pub struct RequestPlaylistCommand;

#[async_trait]
impl Command for RequestPlaylistCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let playlist = state.services().video().get_playlist().await;
        Ok(CommandResult::Effects(vec![Effect::Direct(
            user_id,
            ServerMessage::PlaylistUpdated { playlist },
        )]))
    }
}
