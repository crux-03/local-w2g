use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult, Effect, handler::apply_effect},
    core::AppState,
    services::permissions::Permissions,
    websocket::ServerMessage,
};

pub struct ResumePlaybackCommand;

#[async_trait]
impl Command for ResumePlaybackCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let user = state
            .services()
            .user()
            .get_user(&user_id)
            .await
            .ok_or(crate::Error::InvalidUser)?;

        let message = state
            .services()
            .message()
            .system_log(format!(
                "{} resumed playback",
                user.display_name.unwrap_or(user.id.to_string())
            ))
            .await;

        apply_effect(
            &state,
            Effect::Global(ServerMessage::MessageCreated { entry: message }),
        )
        .await?;
        Ok(ServerMessage::Resume.into())
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_PLAYBACK)
    }
}
