use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult},
    core::AppState,
    services::state::HandshakeOutcome,
    websocket::ServerMessage,
};

pub struct ConfirmReadyForPlayCommand {
    pub request_id: Snowflake,
}

#[async_trait]
impl Command for ConfirmReadyForPlayCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        match state
            .services()
            .state()
            .confirm_play(self.request_id, user_id)
            .await
        {
            HandshakeOutcome::AllConfirmed { video_id } => {
                let video = state
                    .services()
                    .video()
                    .get_entry(video_id)
                    .await
                    .ok_or(crate::Error::InvalidVideo(video_id))?;
                Ok(ServerMessage::Play {
                    request_id: __self.request_id,
                    track_audio: video.audio_track,
                    track_subtitles: video.subtitle_track,
                }
                .into())
            }
            HandshakeOutcome::Pending | HandshakeOutcome::AlreadyResolved => {
                Ok(CommandResult::Silent)
            }
        }
    }
}
