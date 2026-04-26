use std::sync::Arc;

use super::message_parser::parse_client_message;
use crate::{
    commands::{
        Effect,
        handler::{apply_effect, execute_command},
    },
    core::AppState,
    websocket::ServerMessage,
};
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Utf8Bytes, WebSocket},
    },
    http::HeaderMap,
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let username = headers
        .get("X-Username")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());
    tracing::debug!("Client connect attempt with username: {username:?}");
    ws.on_upgrade(move |socket| handle_ws_connection(socket, username, Arc::clone(&state)))
}

async fn handle_ws_connection(socket: WebSocket, username: Option<String>, state: Arc<AppState>) {
    tracing::debug!("Set up socket");
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    
    tracing::debug!("Adding user");
    let user = state.services().user().add_user(username).await;

    // Store connection
    tracing::debug!("Adding connection");
    state.add_connection(user.id, tx).await;

    if let Err(e) = broadcast_user_list(&state).await {
        tracing::warn!("Failed to broadcast user list: {e}")
    }
    if let Err(e) = user_state_system_log(
        &state,
        format!(
            "{} has joined",
            user.display_name.as_ref().unwrap_or(&user.id.to_string())
        ),
    )
    .await
    {
        tracing::warn!("Failed to broadcast event log: {e}")
    }

    // Listen for incoming messages
    let state_clone = state.clone();
    tokio::spawn(async move {
        while let Some(result) = receiver.next().await
            && let Ok(msg) = result
        {
            if let Ok(text) = msg.into_text() {
                match parse_client_message(&text) {
                    Ok(cmd) => {
                        if let Err(e) = execute_command(cmd, user.id, state_clone.clone()).await {
                            let err = json!(ServerMessage::Error {
                                message: e.to_string()
                            });
                            state.send_to_client(&user.id, err.to_string()).await;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse command: {}", e);
                    }
                }
            }
        }
        // Clean up on disconnect

        state_clone.services().user().remove_user(&user.id).await;
        state.services().state().remove_user(user.id).await;
        state_clone.remove_connection(&user.id).await;

        if let Err(e) = broadcast_user_list(&state).await {
            tracing::warn!("Failed to broadcast user list: {e}")
        }
        if let Err(e) = user_state_system_log(
            &state,
            format!(
                "{} has left",
                user.display_name.unwrap_or(user.id.to_string())
            ),
        )
        .await
        {
            tracing::warn!("Failed to broadcast event log: {e}")
        }
    });

    // Send outgoing messages
    while let Some(msg) = rx.recv().await {
        let _ = sender
            .send(axum::extract::ws::Message::Text(Utf8Bytes::from(msg)))
            .await;
    }
}

async fn broadcast_user_list(state: &Arc<AppState>) -> Result<(), crate::Error> {
    let users = state.services().user().get_users().await;
    apply_effect(&state, Effect::Global(ServerMessage::UserList { users })).await?;
    Ok(())
}

async fn user_state_system_log(state: &Arc<AppState>, content: String) -> Result<(), crate::Error> {
    let log = state.services().message().system_log(content).await;
    apply_effect(
        &state,
        Effect::Global(ServerMessage::MessageCreated { entry: log }),
    )
    .await?;
    Ok(())
}
