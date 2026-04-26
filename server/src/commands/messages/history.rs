use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult, Effect},
    core::AppState,
    websocket::ServerMessage,
};

pub struct MessageHistoryCommand;

#[async_trait]
impl Command for MessageHistoryCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let messages = state.services().message().list().await;
        Ok(CommandResult::Effects(vec![Effect::Direct(
            user_id,
            ServerMessage::MessageHistory { history: messages },
        )]))
    }
}
