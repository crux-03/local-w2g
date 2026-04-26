use tauri::{AppHandle, Emitter, Manager};

use crate::{core::AppState, protocol::ServerMessage};

/// Errors arising while dispatching a server message. The dispatcher itself
/// logs these at the call site; this type exists purely to make control flow
/// inside `handle` linear via `?`.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("ws: {0}")]
    Ws(String),
    #[error("mpv: {0}")]
    Mpv(String),
    #[error("file manager: {0}")]
    FileManager(String),
    #[error("emit: {0}")]
    Emit(#[from] tauri::Error),
    #[error("invariant violated: {0}")]
    Invariant(String),
}

pub async fn handle(raw: &str, app: &AppHandle) {
    if let Err(e) = handle_inner(raw, app).await {
        tracing::error!(error = %e, "dispatch failed");
    }
}

async fn handle_inner(raw: &str, app: &AppHandle) -> Result<(), DispatchError> {
    let msg: ServerMessage = serde_json::from_str(raw)?;
    tracing::debug!(?msg, "dispatching");
    let state = app.state::<AppState>();

    match msg {
        ServerMessage::Pong => {
            let elapsed = state.sample_ping().await.elapsed();
            emit(app, "pong", &elapsed.as_micros())?
        }

        ServerMessage::UserIdentity { id } => state.set_client_id(id).await,

        ServerMessage::UserList { users } => emit(app, "user_list", &users)?,

        ServerMessage::PermissionUpdate {
            user_id,
            permissions,
        } => emit(app, "permission_update", &(user_id, permissions))?,

        ServerMessage::MessageHistory { history } => emit(app, "message_history", &history)?,
        ServerMessage::MessageCreated { entry } => emit(app, "message_created", &entry)?,
        ServerMessage::WidgetUpdated { entry } => emit(app, "widget_updated", &entry)?,
        ServerMessage::WidgetDone { entry } => emit(app, "widget_done", &entry)?,

        ServerMessage::RequestResyncReport { id } => {
            let mpv = state.mpv().await.map_err(DispatchError::Mpv)?;
            mpv.pause().await.map_err(DispatchError::Mpv)?; // Resync should pause playback
            let timestamp = mpv.get_time_pos().await.map_err(DispatchError::Mpv)?;

            emit(app, "request_resync_report", &id)?;
            state
                .ws_send(crate::protocol::ClientMessage::SendResyncReport {
                    state_id: id,
                    timestamp,
                })
                .await
                .map_err(DispatchError::Ws)?;
        }

        ServerMessage::CommitResync { timestamp } => {
            let mpv = state.mpv().await.map_err(DispatchError::Mpv)?;
            mpv.seek_absolute(timestamp)
                .await
                .map_err(DispatchError::Mpv)?;
            emit(app, "commit_resync", &timestamp)?;
        }

        ServerMessage::ReadinessUpdated { readiness } => {
            emit(app, "readiness_updated", &readiness)?
        }

        ServerMessage::RequestReadyConfirmation {
            request_id,
            video_id,
            deadline_ms,
        } => {
            // Step 1: assert readiness from local truth.
            let fm = state.fm().await.map_err(DispatchError::FileManager)?;
            let on_device = fm.is_on_device(video_id).await;

            state
                .ws_send(crate::protocol::ClientMessage::AssertReady {
                    video_id,
                    on_device,
                })
                .await
                .map_err(DispatchError::Ws)?;

            emit(
                app,
                "request_ready_confirmation",
                &serde_json::json!({
                    "request_id": request_id,
                    "video_id": video_id,
                    "deadline_ms": deadline_ms,
                }),
            )?;

            // Step 2: if we have the file, load it in mpv and arm the bridge to
            // send ConfirmReadyForPlay on FileLoaded. If we don't, do nothing —
            // the handshake will time out server-side and that's the correct
            // outcome.
            if on_device {
                let path = fm.local_path(video_id).await.ok_or_else(|| {
                    DispatchError::Invariant(
                        "is_on_device returned true but local_path was None".into(),
                    )
                })?;

                let mpv_binary = state.config().read().await.mpv_binary_path.clone();

                state
                    .mpv()
                    .await
                    .map_err(DispatchError::Mpv)?
                    .load_and_confirm(
                        request_id,
                        path,
                        mpv_binary.to_string_lossy().as_ref(),
                        deadline_ms,
                    )
                    .await
                    .map_err(DispatchError::Mpv)?;
            }
        }

        ServerMessage::Play {
            request_id,
            track_audio,
            track_subtitles,
        } => {
            // By the time Play arrives, every client has already confirmed
            // ready, which means mpv is running and the file is loaded.
            // All that's left is to unpause.
            let mpv = state.mpv().await.map_err(DispatchError::Mpv)?;
            mpv.set_audio_track(track_audio)
                .await
                .map_err(DispatchError::Mpv)?;
            mpv.set_subtitle_track(track_subtitles)
                .await
                .map_err(DispatchError::Mpv)?;
            mpv.play().await.map_err(DispatchError::Mpv)?;
            emit(app, "play", &request_id)?;
        }

        ServerMessage::PlayAborted {
            request_id,
            non_confirmers,
        } => emit(
            app,
            "play_aborted",
            &serde_json::json!({
                "request_id": request_id,
                "non_confirmers": non_confirmers,
            }),
        )?,

        ServerMessage::Pause => {
            let mpv = state.mpv().await.map_err(DispatchError::Mpv)?;
            mpv.pause().await.map_err(DispatchError::Mpv)?;
            emit(app, "pause", &true)?;
        }
        ServerMessage::Resume => {
            let mpv = state.mpv().await.map_err(DispatchError::Mpv)?;
            mpv.play().await.map_err(DispatchError::Mpv)?;
            emit(app, "resume", &true)?;
        }
        ServerMessage::Seek { timestamp } => {
            let mpv = state.mpv().await.map_err(DispatchError::Mpv)?;
            mpv.seek_absolute(timestamp)
                .await
                .map_err(DispatchError::Mpv)?;
            emit(app, "seek", &true)?;
        }

        ServerMessage::VideoSelected { video_id } => emit(app, "video_selected", &video_id)?,
        ServerMessage::PlaylistUpdated { playlist } => {
            let fm = state.fm().await.map_err(DispatchError::FileManager)?;
            let mut on_device = Vec::new();
            for e in playlist.iter() {
                if fm.is_on_device(e.id).await {
                    on_device.push(e.id);
                }
            }

            state
                .ws_send(crate::protocol::ClientMessage::AssertReadyBulk { on_device })
                .await
                .map_err(|e| DispatchError::Ws(e))?;

            emit(app, "playlist_updated", &playlist)?;
        }

        ServerMessage::Error { message } => {
            tracing::warn!(%message, "server reported error");
            emit(app, "server_error", &message)?;
        }
    }

    Ok(())
}

fn emit<T: serde::Serialize + Clone>(
    app: &AppHandle,
    event: &str,
    payload: &T,
) -> Result<(), tauri::Error> {
    app.emit(event, payload.clone())
}
