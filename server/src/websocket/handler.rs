use std::sync::Arc;

use super::message_parser::parse_client_message;
use crate::{commands::handler::execute_command, core::AppState, websocket::ServerMessage};
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Utf8Bytes, WebSocket},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, Arc::clone(&state)))
}

async fn handle_ws_connection(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let user = state.services().user().add_user(None).await;

    // Store connection
    state.add_connection(user.id, tx).await;

    // Listen for incoming messages
    let state_clone = state.clone();
    tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            if let Ok(msg) = result {
                if let Ok(text) = msg.into_text() {
                    match parse_client_message(&text) {
                        Ok(cmd) => {
                            if let Err(e) = execute_command(cmd, user.id, state_clone.clone()).await
                            {
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
        }
        // Clean up on disconnect

        state_clone.services().user().remove_user(&user.id).await;
        state.services().state().remove_user(user.id).await;
        state_clone.remove_connection(&user.id).await;
    });

    // Send outgoing messages
    while let Some(msg) = rx.recv().await {
        let _ = sender
            .send(axum::extract::ws::Message::Text(Utf8Bytes::from(msg)))
            .await;
    }
}
