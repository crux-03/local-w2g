use tauri::State;

use crate::core::AppState;
use crate::protocol::{ClientMessage, Snowflake};
use crate::CommandResult;

#[tauri::command]
pub async fn init_mpv_manager(state: State<'_, AppState>) -> CommandResult<()> {
    tracing::info!("Initializing mpv manager");
    state
        .init_mpv_manager()
        .await
        .inspect_err(|e| tracing::error!(%e, "init_mpv_manager command failed"))?;
    tracing::info!("Initialized mpv manager");
    Ok(())
}

#[tauri::command]
pub async fn play(state: State<'_, AppState>) -> Result<(), String> {
    state.ws_send(ClientMessage::Play).await
}

#[tauri::command]
pub async fn select_video(video_id: Snowflake, state: State<'_, AppState>) -> Result<(), String> {
    state.ws_send(ClientMessage::SelectVideo { video_id }).await
}

#[tauri::command]
pub async fn pause(state: State<'_, AppState>) -> Result<(), String> {
    state.ws_send(ClientMessage::RequestPause).await
}

#[tauri::command]
pub async fn resume(state: State<'_, AppState>) -> Result<(), String> {
    state.ws_send(ClientMessage::RequestResume).await
}
