use std::sync::Arc;

use async_trait::async_trait;

use crate::{Snowflake, commands::{Command, CommandResult}, core::AppState, websocket::ServerMessage};

pub struct AssertReadyCommand {
    pub video_id: Snowflake,
    pub on_device: bool,
}

#[async_trait]
impl Command for AssertReadyCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let Some(view) = state
            .services()
            .state()
            .assert_ready(user_id, self.video_id, self.on_device)
            .await
        else {
            return Ok(CommandResult::Silent);
        };

        Ok(ServerMessage::ReadinessUpdated { readiness: view }.into())
    }
}