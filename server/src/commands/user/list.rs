use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult},
    core::AppState,
    websocket::ServerMessage,
};

pub struct ListUsersCommand;

#[async_trait]
impl Command for ListUsersCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        _user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let users = state.services().user().get_users().await;

        Ok(ServerMessage::UserList { users }.into())
    }
}
