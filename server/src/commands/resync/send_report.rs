use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{BroadcastScope, Command, CommandResult},
    core::AppState,
    services::permissions::Permissions,
    websocket::ServerMessage,
};

pub struct SendResyncReportCommand {
    pub state_id: Snowflake,
    pub timestamp: u32,
}

#[async_trait]
impl Command for SendResyncReportCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        tracing::info!("Entered command");
        let completed = state
            .services()
            .playback()
            .resync_report(self.state_id, user_id, self.timestamp)
            .await?;

        if completed {
            tracing::info!("State was completed");
            let collapsed = state
                .services()
                .playback()
                .collapse_resync(self.state_id)
                .await?;
            return Ok(CommandResult::Broadcast(ServerMessage::CommitResync {
                timestamp: collapsed.timestamp,
            }));
        }

        Ok(CommandResult::Silent)
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::SEND_STATE)
    }

    fn broadcast_scope(&self) -> BroadcastScope {
        BroadcastScope::Global
    }
}
