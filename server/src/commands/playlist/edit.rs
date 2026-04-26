use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult, Effect, handler::apply_effect},
    core::AppState,
    services::permissions::Permissions,
    websocket::ServerMessage,
};

pub struct SetEntryDisplayNameCommand {
    pub video_id: Snowflake,
    pub display_name: String,
}

#[async_trait]
impl Command for SetEntryDisplayNameCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let _ = state
            .services()
            .video()
            .set_title(self.video_id, self.display_name.clone())
            .await?;
        let playlist = state.services().video().get_playlist().await;

        system_log(&state, user_id, "updated a |-display name-|").await?;

        Ok(ServerMessage::PlaylistUpdated { playlist }.into())
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_MEDIA)
    }
}

pub struct SetEntryAudioTrackCommand {
    pub video_id: Snowflake,
    pub audio_track: i32,
}

#[async_trait]
impl Command for SetEntryAudioTrackCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let _ = state
            .services()
            .video()
            .set_audio_track(self.video_id, self.audio_track)
            .await?;
        let playlist = state.services().video().get_playlist().await;

        system_log(&state, user_id, "updated an |-audio track-|").await?;

        Ok(ServerMessage::PlaylistUpdated { playlist }.into())
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_MEDIA)
    }
}

pub struct SetEntrySubtitleTrackCommand {
    pub video_id: Snowflake,
    pub subtitle_track: i32,
}

#[async_trait]
impl Command for SetEntrySubtitleTrackCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let _ = state
            .services()
            .video()
            .set_subtitle_track(self.video_id, self.subtitle_track)
            .await?;
        let playlist = state.services().video().get_playlist().await;

        system_log(&state, user_id, "updated a |-subtitle track-|").await?;

        Ok(ServerMessage::PlaylistUpdated { playlist }.into())
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_MEDIA)
    }
}

async fn system_log(
    state: &AppState,
    user_id: Snowflake,
    action: &str,
) -> Result<(), crate::Error> {
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
            "[ENTRY_EDIT] {} {action}",
            user.display_name.unwrap_or(user.id.to_string())
        ))
        .await;

    apply_effect(
        &state,
        Effect::Global(ServerMessage::MessageCreated { entry: message }),
    )
    .await?;
    Ok(())
}
