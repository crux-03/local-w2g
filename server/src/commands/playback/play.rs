use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Snowflake,
    commands::{Command, CommandResult, Effect, handler::apply_effect},
    core::AppState,
    services::{permissions::Permissions, state::PLAY_HANDSHAKE_TIMEOUT},
    websocket::ServerMessage,
};

pub struct PlayCommand;

#[async_trait]
impl Command for PlayCommand {
    async fn execute(
        &self,
        state: Arc<AppState>,
        _user_id: Snowflake,
    ) -> Result<CommandResult, crate::Error> {
        let video_id = state
            .services()
            .playback()
            .current()
            .await
            .ok_or(crate::Error::NoVideoSelected)?;

        let request_id = state.services().snowflake().generate();
        state
            .services()
            .state()
            .begin_play_handshake(request_id, video_id)
            .await;

        // Fire-and-forget timeout. Owns its own Arc; no join needed.
        let state_for_timer = Arc::clone(&state);
        tokio::spawn(async move {
            tokio::time::sleep(PLAY_HANDSHAKE_TIMEOUT).await;
            if let Some(missing) = state_for_timer
                .services()
                .state()
                .timeout_play(request_id)
                .await
            {
                let _ = apply_effect(
                    &state_for_timer,
                    Effect::Global(ServerMessage::PlayAborted {
                        request_id,
                        non_confirmers: missing,
                    }),
                )
                .await;
            }
        });

        Ok(ServerMessage::RequestReadyConfirmation {
            request_id,
            video_id, // include so clients know what they're confirming
            deadline_ms: PLAY_HANDSHAKE_TIMEOUT.as_millis() as u64,
        }
        .into())
    }

    fn required_permission(&self) -> Option<Permissions> {
        Some(Permissions::MANAGE_PLAYBACK)
    }
}
