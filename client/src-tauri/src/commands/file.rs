use std::{path::Path, str::FromStr};

use tauri::State;
use url::Url;

use crate::{core::AppState, protocol::Snowflake, CommandResult};

#[tauri::command]
pub async fn init_file_manager(state: State<'_, AppState>) -> CommandResult<()> {
    tracing::info!("Initializing file manager");
    let videos_dir = state.config().read().await.videos_directory.clone();
    state.init_file_manager(videos_dir).await?;
    tracing::info!("File manager initialized");
    Ok(())
}

#[tauri::command]
pub async fn file_on_device(state: State<'_, AppState>, id: Snowflake) -> CommandResult<bool> {
    Ok(state.fm().await?.is_on_device(id).await)
}

#[tauri::command]
pub async fn load_local_files(state: State<'_, AppState>) -> CommandResult<Vec<Snowflake>> {
    Ok(state.fm().await?.on_device_set().await)
}

#[tauri::command]
pub async fn download_file(
    state: State<'_, AppState>,
    video_id: Snowflake,
    display_name: String,
) -> Result<(), String> {
    let extension = Path::new(&display_name)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or("display name has no extension")?;

    let raw_url = state.config().read().await.server_url.clone();
    let server_url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        raw_url
    } else {
        format!("http://{}", raw_url)
    };

    let client_id = state.client_id().lock().await.ok_or("client id not set")?;

    let url = Url::from_str(&format!(
        "{server_url}/api/v1/videos/{video_id}?client_id={client_id}"
    ))
    .map_err(|e| format!("Error building url: {e}"))?;

    state
        .fm()
        .await?
        .start_download(video_id, url, extension)
        .await;
    Ok(())
}
