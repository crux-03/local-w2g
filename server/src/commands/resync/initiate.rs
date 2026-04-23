use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::{
    Snowflake,
    commands::{BroadcastScope, Command, CommandResult},
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
        let user_service = state.services().user();
        let users = user_service
            .get_users()
            .await
            .iter()
            .map(|u| u.id)
            .collect();

        log_initiate(Arc::clone(&state), user_id).await?;

        let state_id = state.services().playback().initiate_resync(users).await;

        Ok(CommandResult::Broadcast(
            ServerMessage::RequestResyncReport { id: state_id },
        ))
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_PLAYBACK)
    }

    fn broadcast_scope(&self) -> BroadcastScope {
        BroadcastScope::Global
    }
}

async fn log_initiate(state: Arc<AppState>, user_id: Snowflake) -> Result<(), crate::Error> {
    let username = state
        .services()
        .user()
        .get_user(&user_id)
        .await
        .ok_or(crate::Error::InvalidUser)?
        .display_name
        .unwrap_or(user_id.to_string());

    let log_entry = state
        .services()
        .message()
        .system_log(format!("{username} initiated a Resync request"))
        .await;

    state.broadcast(json!(log_entry).to_string()).await;
    Ok(())
}
