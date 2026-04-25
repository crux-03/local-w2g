use tauri::{AppHandle, Emitter, Manager};

use crate::{core::AppState, protocol::ServerMessage};

pub async fn handle(raw: &str, app: &AppHandle) {
    let msg: ServerMessage = match serde_json::from_str(raw) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(error = %e, raw, "failed to parse server message");
            return;
        }
    };

    let state = app.state::<AppState>();

    match msg {
        ServerMessage::Pong => {
            emit(app, "pong", &String::new());
        }
        ServerMessage::UserIdentity { id } => {
            state.set_client_id(id).await;
        }
        ServerMessage::UserList { users } => {
            emit(app, "user_list", &users);
        }
        ServerMessage::PermissionUpdate {
            user_id,
            permissions,
        } => emit(app, "permission_update", &(user_id, permissions)),
        ServerMessage::MessageHistory { history } => {
            emit(app, "message_history", &history);
        }
        ServerMessage::MessageCreated { entry } => {
            emit(app, "message_created", &entry);
        }
        ServerMessage::WidgetUpdated { entry } => {
            emit(app, "widget_updated", &entry);
        }
        ServerMessage::WidgetDone { entry } => {
            emit(app, "widget_done", &entry);
        }

        ServerMessage::RequestResyncReport { id } => {
            // TODO: gather local state, send SendResyncReport via WsHandle.
            emit(app, "request_resync_report", &id);
        }
        ServerMessage::CommitResync { timestamp } => {
            emit(app, "commit_resync", &timestamp);
        }

        ServerMessage::ReadinessUpdated { readiness } => {
            emit(app, "readiness_updated", &readiness);
        }
        ServerMessage::RequestReadyConfirmation {
            request_id,
            video_id,
            deadline_ms,
        } => {
            // TODO: check local library; if on device, auto-confirm.
            emit(
                app,
                "request_ready_confirmation",
                &serde_json::json!({
                    "request_id": request_id,
                    "video_id": video_id,
                    "deadline_ms": deadline_ms,
                }),
            );
        }

        ServerMessage::Play { request_id } => {
            emit(app, "play", &request_id);
        }
        ServerMessage::PlayAborted {
            request_id,
            non_confirmers,
        } => {
            emit(
                app,
                "play_aborted",
                &serde_json::json!({
                    "request_id": request_id,
                    "non_confirmers": non_confirmers,
                }),
            );
        }
        ServerMessage::VideoSelected { video_id } => {
            emit(app, "video_selected", &video_id);
        }
        ServerMessage::PlaylistUpdated { playlist } => {
            emit(app, "playlist_updated", &playlist);
        }

        ServerMessage::Error { message } => {
            tracing::warn!(%message, "server reported error");
            emit(app, "server_error", &message);
        }
    }
}

fn emit<T: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: &T) {
    if let Err(e) = app.emit(event, payload.clone()) {
        tracing::error!(error = %e, event, "failed to emit");
    }
}
