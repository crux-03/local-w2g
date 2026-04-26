use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult, Effect, handler::apply_effect},
    core::AppState,
    services::permissions::Permissions,
    websocket::ServerMessage,
};

pub struct SeekCommand {
    pub timestamp: f64,
}

#[async_trait]
impl Command for SeekCommand {
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
                "{} set playback position to |-{}-|",
                user.display_name.unwrap_or(user.id.to_string()),
                format_duration(self.timestamp)
            ))
            .await;

        apply_effect(
            &state,
            Effect::Global(ServerMessage::MessageCreated { entry: message }),
        )
        .await?;

        Ok(ServerMessage::Seek {
            timestamp: self.timestamp,
        }
        .into())
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_PLAYBACK)
    }
}

fn format_duration(seconds: f64) -> String {
    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}
