use std::sync::Arc;

use crate::Snowflake;
use async_trait::async_trait;

use crate::{
    commands::{Command, CommandResult},
    core::AppState,
    services::permissions::Permissions,
    websocket::ServerMessage,
};

pub struct SendMessageCommand {
    pub content: String,
}

#[async_trait]
impl Command for SendMessageCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let entry = state
            .services()
            .message()
            .user_message(user_id, self.content.clone())
            .await;
        Ok(ServerMessage::MessageCreated { entry }.into())
    }

    fn validate(&self) -> Result<(), crate::Error> {
        if self.content.is_empty() || self.content.len() > 2000 {
            return Err(crate::Error::CommandValidation(
                "send_message".into(),
                "Message must be 1-2000 characters".into(),
            ));
        }
        Ok(())
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::SEND_MESSAGE)
    }
}
