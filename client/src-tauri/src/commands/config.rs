use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::{core::AppState, CommandResult};

#[tauri::command]
pub async fn load_username(app: AppHandle) -> String {
    tracing::info!("Loading username");
    app.state::<AppState>()
        .config()
        .read()
        .await
        .username
        .clone()
}

#[tauri::command]
pub async fn load_server_url(app: AppHandle) -> String {
    tracing::info!("Loading server url");
    app.state::<AppState>()
        .config()
        .read()
        .await
        .server_url
        .clone()
}

#[tauri::command]
pub async fn load_mpv_binary(app: AppHandle) -> String {
    tracing::info!("loading mpv binary");
    app.state::<AppState>()
        .config()
        .read()
        .await
        .mpv_binary_path
        .to_str()
        .unwrap_or_default()
        .to_string()
        .clone()
}

#[tauri::command]
pub async fn load_videos_dir(app: AppHandle) -> String {
    tracing::info!("Loading videos dir");
    app.state::<AppState>()
        .config()
        .read()
        .await
        .videos_directory
        .to_str()
        .unwrap_or_default()
        .to_string()
        .clone()
}

#[tauri::command]
pub async fn set_mpv_binary(app: AppHandle, path: String) -> CommandResult<()> {
    app.state::<AppState>().set_mpv_binary(&app, path).await
}

#[tauri::command]
pub async fn set_videos_dir(app: AppHandle, path: String) -> CommandResult<()> {
    app.state::<AppState>().set_videos_dir(&app, path).await
}

#[tauri::command]
pub async fn password_to_clipboard(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    let pw = state.password().lock().await;
    app.clipboard()
        .write_text(pw.clone())
        .map_err(|e| format!("Error when writing to clipboard: {e}"))
}
