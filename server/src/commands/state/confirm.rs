use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult},
    core::AppState,
    services::state::HandshakeOutcome,
    websocket::ServerMessage,
};

pub struct ConfirmReadyForPlayCommand {
    pub request_id: Snowflake,
}

#[async_trait]
impl Command for ConfirmReadyForPlayCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        match state
            .services()
            .state()
            .confirm_play(self.request_id, user_id)
            .await
        {
            HandshakeOutcome::AllConfirmed { video_id } => Ok(ServerMessage::Play {
                request_id: self.request_id,
                video_id,
            }
            .into()),
            HandshakeOutcome::Pending | HandshakeOutcome::AlreadyResolved => {
                Ok(CommandResult::Silent)
            }
        }
    }
}
