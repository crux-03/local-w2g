use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult, Effect},
    core::AppState,
    websocket::ServerMessage,
};

pub struct IdentifySelfCommand;

#[async_trait]
impl Command for IdentifySelfCommand {
    async fn execute(
        &self,
        _state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        Ok(CommandResult::Effects(vec![Effect::Direct(
            user_id,
            ServerMessage::UserIdentity { id: user_id },
        )]))
    }
}
