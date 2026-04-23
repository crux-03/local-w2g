use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult, Effect},
    core::AppState,
    services::permissions::Permissions,
    websocket::ServerMessage,
};

pub struct InitiateResyncCommand;

#[async_trait]
impl Command for InitiateResyncCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let username = state
            .services()
            .user()
            .get_user(&user_id)
            .await
            .ok_or(crate::Error::InvalidUser)?
            .display_name
            .unwrap_or_else(|| user_id.to_string());

        let log_entry = state
            .services()
            .message()
            .system_log(format!("{username} initiated a Resync request"))
            .await;

        let user_ids = state
            .services()
            .user()
            .get_users()
            .await
            .iter()
            .map(|u| u.id)
            .collect();

        let state_id = state.services().playback().initiate_resync(user_ids).await;

        Ok(CommandResult::Effects(vec![
            Effect::Global(ServerMessage::MessageCreated { entry: log_entry }),
            Effect::Global(ServerMessage::RequestResyncReport { id: state_id }),
        ]))
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_PLAYBACK)
    }
}
