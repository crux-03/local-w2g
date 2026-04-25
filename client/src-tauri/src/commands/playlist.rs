use tauri::State;

use crate::{core::AppState, protocol::ClientMessage, CommandResult};

#[tauri::command]
pub async fn request_playlist(state: State<'_, AppState>) -> CommandResult<()> {
    state.ws_send(ClientMessage::RequestPlaylist).await
}
