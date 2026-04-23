use std::sync::Arc;

use async_trait::async_trait;

use crate::{Snowflake, commands::{Command, CommandResult}, core::AppState};

pub struct HeartbeatCommand;

#[async_trait]
impl Command for HeartbeatCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        state.services().state().heartbeat(user_id).await;
        Ok(CommandResult::Silent)
    }
}