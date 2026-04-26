use tauri::State;

use crate::core::AppState;
use crate::protocol::ClientMessage;

#[tauri::command]
pub async fn send_chat_message(content: String, state: State<'_, AppState>) -> Result<(), String> {
    state.ws_send(ClientMessage::SendMessage { content }).await
}

#[tauri::command]
pub async fn request_message_history(state: State<'_, AppState>) -> Result<(), String> {
    state.ws_send(ClientMessage::RequestMessageHistory).await
}
