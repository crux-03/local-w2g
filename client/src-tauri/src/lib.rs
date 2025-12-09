mod mpv;
mod types;
mod websocket;

use mpv::MpvManager;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::RwLock;
use types::{ClientMessage, CommandResult, Video, LogEntry, LogSource};
use websocket::WebSocketClient;
use std::{path::PathBuf, sync::Arc, time::Duration};
use chrono::Utc;

// Helper function to emit client-side logs
fn emit_client_log(app: &AppHandle, action: String) {
    let log = LogEntry {
        timestamp: Utc::now(),
        user_id: "local".to_string(),
        username: Some("Client".to_string()),
        action,
        source: LogSource::Client,
    };
    let _ = app.emit("client-log", log);
}

pub struct AppState {
    ws_client: Arc<RwLock<WebSocketClient>>,
    mpv_manager: Arc<RwLock<MpvManager>>,
    config: Arc<RwLock<ClientConfig>>,
    client_id: Arc<RwLock<Option<String>>>,
    is_owner: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ClientConfig {
    server_url: Option<String>,
    video_storage_path: Option<String>,
    mpv_binary_path: Option<String>,
}

impl AppState {
    fn new() -> Self {
        Self {
            ws_client: Arc::new(RwLock::new(WebSocketClient::new())),
            mpv_manager: Arc::new(RwLock::new(MpvManager::new())),
            config: Arc::new(RwLock::new(ClientConfig {
                server_url: None,
                video_storage_path: None,
                mpv_binary_path: None,
            })),
            client_id: Arc::new(RwLock::new(None)),
            is_owner: Arc::new(RwLock::new(false)),
        }
    }
}

// ============================================================================
// Configuration Commands
// ============================================================================

#[tauri::command]
async fn set_config(
    server_url: Option<String>,
    video_path: Option<String>,
    mpv_path: Option<String>,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    let mut config = state.config.write().await;
    
    if let Some(url) = server_url {
        config.server_url = Some(url);
    }
    if let Some(path) = video_path {
        config.video_storage_path = Some(path);
    }
    if let Some(path) = mpv_path {
        config.mpv_binary_path = Some(path);
    }
    
    Ok(())
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> CommandResult<ClientConfig> {
    let config = state.config.read().await;
    Ok(config.clone())
}

// ============================================================================
// File Dialog Commands
// ============================================================================

#[tauri::command]
async fn pick_file(
    filters: Option<Vec<(String, Vec<String>)>>,
) -> CommandResult<Option<String>> {
    let task = tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();
        
        // Add filters if provided
        if let Some(filter_list) = filters {
            for (name, extensions) in filter_list {
                dialog = dialog.add_filter(&name, &extensions);
            }
        }
        
        dialog.pick_file().map(|path| path.to_string_lossy().to_string())
    });
    
    match task.await {
        Ok(result) => Ok(result),
        Err(_) => Err("File dialog failed".to_string()),
    }
}

#[tauri::command]
async fn pick_folder() -> CommandResult<Option<String>> {
    let task = tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .pick_folder()
            .map(|path| path.to_string_lossy().to_string())
    });
    
    match task.await {
        Ok(result) => Ok(result),
        Err(_) => Err("Folder dialog failed".to_string()),
    }
}

#[tauri::command]
async fn save_file(
    default_name: Option<String>,
) -> CommandResult<Option<String>> {
    let task = tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();
        
        if let Some(name) = default_name {
            dialog = dialog.set_file_name(&name);
        }
        
        dialog.save_file().map(|path| path.to_string_lossy().to_string())
    });
    
    match task.await {
        Ok(result) => Ok(result),
        Err(_) => Err("Save dialog failed".to_string()),
    }
}

// ============================================================================
// WebSocket Commands
// ============================================================================

#[tauri::command]
async fn connect(
    server_url: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> CommandResult<()> {
    emit_client_log(&app_handle, format!("Connecting to {}", server_url));
    
    // Update config
    state.config.write().await.server_url = Some(server_url.clone());
    
    // Connect to WebSocket
    let mut ws_client = state.ws_client.write().await;
    
    // Pass state references for storing client_id and is_owner
    ws_client.connect(
        &server_url, 
        app_handle.clone(),
        state.client_id.clone(),
        state.is_owner.clone(),
    ).await?;
    
    emit_client_log(&app_handle, "Connected successfully".to_string());
    
    Ok(())
}

#[tauri::command]
async fn disconnect(state: State<'_, AppState>, app_handle: AppHandle) -> CommandResult<()> {
    emit_client_log(&app_handle, "Disconnecting from server".to_string());
    
    let mut ws_client = state.ws_client.write().await;
    ws_client.disconnect().await?;
    
    // Clear client state
    *state.client_id.write().await = None;
    *state.is_owner.write().await = false;
    
    // Also stop mpv
    let mut mpv = state.mpv_manager.write().await;
    mpv.stop().await?;
    
    emit_client_log(&app_handle, "Disconnected successfully".to_string());
    
    Ok(())
}

#[tauri::command]
async fn is_connected(state: State<'_, AppState>) -> CommandResult<bool> {
    let ws_client = state.ws_client.read().await;
    Ok(ws_client.is_connected().await)
}

#[tauri::command]
async fn get_client_info(state: State<'_, AppState>) -> CommandResult<serde_json::Value> {
    let client_id = state.client_id.read().await;
    let is_owner = state.is_owner.read().await;
    
    Ok(serde_json::json!({
        "client_id": *client_id,
        "is_owner": *is_owner,
    }))
}

#[tauri::command]
async fn send_message(
    message_type: String,
    data: Option<serde_json::Value>,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    let message = match message_type.as_str() {
        "pause" => ClientMessage::Pause,
        "play" => ClientMessage::Play,
        "seek" => {
            let position = data
                .and_then(|d| d.as_f64())
                .ok_or("Missing or invalid position for seek")?;
            ClientMessage::Seek { position }
        }
        "subtitle_track" => {
            let index = data
                .and_then(|d| d.as_u64())
                .map(|i| i as u32)
                .ok_or("Missing or invalid index for subtitle_track")?;
            ClientMessage::SubtitleTrack { index }
        }
        "audio_track" => {
            let index = data
                .and_then(|d| d.as_u64())
                .map(|i| i as u32)
                .ok_or("Missing or invalid index for audio_track")?;
            ClientMessage::AudioTrack { index }
        }
        "ready" => {
            let value = data
                .and_then(|d| d.as_bool())
                .ok_or("Missing or invalid value for ready")?;
            ClientMessage::Ready { value }
        }
        "set_username" => {
            let username = data
                .and_then(|d| d.as_str().map(|s| s.to_string()))
                .ok_or("Missing or invalid username")?;
            ClientMessage::SetUsername { username }
        }
        "set_permission" => {
            let obj = data.ok_or("Missing data for set_permission")?;
            let client_id = obj.get("client_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing client_id")?
                .to_string();
            let permission = obj.get("permission")
                .and_then(|v| v.as_str())
                .ok_or("Missing permission")?
                .to_string();
            let value = obj.get("value")
                .and_then(|v| v.as_bool())
                .ok_or("Missing value")?;
            
            ClientMessage::SetPermission { client_id, permission, value }
        }
        "transfer_ownership" => {
            let client_id = data
                .and_then(|d| d.as_str().map(|s| s.to_string()))
                .ok_or("Missing or invalid client_id")?;
            ClientMessage::TransferOwnership { client_id }
        }
        "select_video" => {
            let index = data
                .and_then(|d| d.as_u64())
                .map(|i| i as usize)
                .ok_or("Missing or invalid index for select_video")?;
            ClientMessage::SelectVideo { index }
        }
        "request_state" => ClientMessage::RequestState,
        _ => return Err(format!("Unknown message type: {}", message_type)),
    };

    let ws_client = state.ws_client.read().await;
    ws_client.send(message).await?;

    Ok(())
}

// ============================================================================
// Video Upload/Download Commands
// ============================================================================

#[tauri::command]
async fn upload_video(
    file_path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> CommandResult<Video> {
    // Check if connected
    let config = state.config.read().await;
    let server_url = config.server_url.as_ref()
        .ok_or("Not connected to server")?;

    // Check if owner
    let is_owner = *state.is_owner.read().await;
    if !is_owner {
        return Err("Only the owner can upload videos".to_string());
    }

    // Get client ID
    let client_id = state.client_id.read().await
        .as_ref()
        .ok_or("Not connected")?
        .clone();

    // Read file
    let file_bytes = tokio::fs::read(&file_path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let file_name = PathBuf::from(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?
        .to_string();

    emit_client_log(&app_handle, format!("Uploading video: {}", file_name));

    let mime_type = match file_path.to_lowercase().as_str() {
        p if p.ends_with(".mp4") => "video/mp4",
        p if p.ends_with(".mov") => "video/quicktime",
        p if p.ends_with(".avi") => "video/x-msvideo",
        p if p.ends_with(".mkv") => "video/x-matroska",
        p if p.ends_with(".webm") => "video/webm",
        _ => "application/octet-stream",  // fallback
    };

    // Create multipart form
    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name.clone())
        .mime_str(mime_type)
        .map_err(|e| format!("Failed to set mime type: {}", e))?;

    let form = reqwest::multipart::Form::new()
        .part("file", part);

    // Send upload request
    let base_url = server_url
        .trim_end_matches("/")
        .trim_end_matches("/ws");

    let base_url = if !base_url.starts_with("http") {
        format!("http://{}", base_url)
    } else {
        base_url.to_string()
    };

    let upload_url = format!("{}/upload?client_id={}", base_url, client_id);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to build reqwest client: {e}"))?;

    let response = client
        .post(&upload_url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Upload failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        emit_client_log(&app_handle, format!("Upload failed: {}", error_text));
        return Err(format!("Upload failed: {}", error_text));
    }

    let upload_result: serde_json::Value = response.json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // The server will broadcast VideoUploaded message, but we can also return the video info
    let video = Video {
        id: upload_result.get("video_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        filename: upload_result.get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        size_bytes: upload_result.get("size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        size_display: format_size(upload_result.get("size").and_then(|v| v.as_u64()).unwrap_or(0)),
        uploaded_at: chrono::Utc::now(),
        uploader_id: client_id,
    };

    emit_client_log(&app_handle, format!("Upload complete: {} ({})", file_name, video.size_display));

    Ok(video)
}

#[tauri::command]
async fn get_video_url(
    video_id: String,
    download: Option<bool>,
    state: State<'_, AppState>,
) -> CommandResult<String> {
    let config = state.config.read().await;
    let server_url = config.server_url.as_ref().ok_or("Server url not set")?;
    let base_url = server_url
        .trim_end_matches("/")
        .trim_end_matches("/ws");

    let base_url = if !base_url.starts_with("http") {
        format!("http://{}", base_url)
    } else {
        base_url.to_string()
    };
    
    let download_param = if download.unwrap_or(false) { "true" } else { "false" };
    Ok(format!("{}/video/{}?download={}", base_url, video_id, download_param))
}

#[tauri::command]
async fn download_video(
    video_id: String,
    save_path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> CommandResult<()> {
    emit_client_log(&app_handle, format!("Downloading video: {}", video_id));
    
    let video_url = get_video_url(video_id.clone(), Some(true), state.clone()).await?;
    
    let client = reqwest::Client::new();
    let response = client.get(&video_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    
    if !response.status().is_success() {
        emit_client_log(&app_handle, format!("Download failed with status: {}", response.status()));
        return Err(format!("Download failed with status: {}", response.status()));
    }
    
    let bytes = response.bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    emit_client_log(&app_handle, format!("Saving to: {}", save_path));
    
    tokio::fs::write(&save_path, bytes)
        .await
        .map_err(|e| format!("Failed to save file: {}", e))?;
    
    emit_client_log(&app_handle, "Download complete".to_string());
    
    Ok(())
}

#[tauri::command]
async fn download_video_to_storage(
    video_id: String,
    filename: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> CommandResult<String> {
    // Get video storage path from config
    let config = state.config.read().await;
    let storage_path = config.video_storage_path.as_ref()
        .ok_or("Video storage path not configured")?;
    
    // Create full path
    let video_path = std::path::Path::new(storage_path).join(&filename);
    let video_path_str = video_path.to_string_lossy().to_string();
    
    // Download the video
    download_video(video_id.clone(), video_path_str.clone(), state.clone(), app_handle.clone()).await?;
    
    // Automatically set ready status after download
    emit_client_log(&app_handle, "Setting ready status".to_string());
    
    let ws_client = state.ws_client.read().await;
    ws_client.send(ClientMessage::Ready { value: true }).await
        .map_err(|e| format!("Failed to set ready: {}", e))?;
    
    Ok(video_path_str)
}

#[tauri::command]
async fn check_video_downloaded(
    filename: String,
    state: State<'_, AppState>,
) -> CommandResult<Option<String>> {
    let config = state.config.read().await;
    let storage_path = config.video_storage_path.as_ref()
        .ok_or("Video storage path not configured")?;
    
    let video_path = std::path::Path::new(storage_path).join(&filename);
    
    if video_path.exists() {
        Ok(Some(video_path.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

// ============================================================================
// MPV Commands
// ============================================================================

#[tauri::command]
async fn start_mpv(
    video_path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> CommandResult<()> {
    let config = state.config.read().await;
    let mpv_path = config
        .mpv_binary_path
        .as_ref()
        .ok_or("MPV binary path not configured")?;

    emit_client_log(&app_handle, format!("Starting MPV with local file: {}", video_path));
    
    let mut mpv = state.mpv_manager.write().await;
    mpv.start(mpv_path, &video_path).await?;

    emit_client_log(&app_handle, "MPV started successfully".to_string());

    Ok(())
}

#[tauri::command]
async fn start_mpv_with_url(
    video_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> CommandResult<()> {
    emit_client_log(&app_handle, format!("Fetching video URL for: {}", video_id));
    
    let video_url = get_video_url(video_id, Some(false), state.clone()).await?;
    
    let config = state.config.read().await;
    let mpv_path = config
        .mpv_binary_path
        .as_ref()
        .ok_or("MPV binary path not configured")?;

    emit_client_log(&app_handle, format!("Starting MPV with stream: {}", video_url));
    
    let mut mpv = state.mpv_manager.write().await;
    mpv.start(mpv_path, &video_url).await?;

    emit_client_log(&app_handle, "MPV started successfully".to_string());

    Ok(())
}

#[tauri::command]
async fn stop_mpv(state: State<'_, AppState>) -> CommandResult<()> {
    let mut mpv = state.mpv_manager.write().await;
    mpv.stop().await?;
    Ok(())
}

#[tauri::command]
async fn mpv_pause(state: State<'_, AppState>) -> CommandResult<()> {
    let mpv = state.mpv_manager.read().await;
    mpv.pause().await?;
    
    // Also send to server
    let ws_client = state.ws_client.read().await;
    ws_client.send(ClientMessage::Pause).await?;
    
    Ok(())
}

#[tauri::command]
async fn mpv_play(state: State<'_, AppState>) -> CommandResult<()> {
    let mpv = state.mpv_manager.read().await;
    mpv.play().await?;
    
    // Also send to server
    let ws_client = state.ws_client.read().await;
    ws_client.send(ClientMessage::Play).await?;
    
    Ok(())
}

#[tauri::command]
async fn mpv_seek(position: f64, state: State<'_, AppState>) -> CommandResult<()> {
    let mpv = state.mpv_manager.read().await;
    mpv.seek(position).await?;
    
    // Also send to server
    let ws_client = state.ws_client.read().await;
    ws_client.send(ClientMessage::Seek { position }).await?;
    
    Ok(())
}

#[tauri::command]
async fn mpv_set_subtitle_track(index: u32, state: State<'_, AppState>) -> CommandResult<()> {
    let mpv = state.mpv_manager.read().await;
    mpv.set_subtitle_track(index).await?;
    
    // Also send to server
    let ws_client = state.ws_client.read().await;
    ws_client.send(ClientMessage::SubtitleTrack { index }).await?;
    
    Ok(())
}

#[tauri::command]
async fn mpv_set_audio_track(index: u32, state: State<'_, AppState>) -> CommandResult<()> {
    let mpv = state.mpv_manager.read().await;
    mpv.set_audio_track(index).await?;
    
    // Also send to server
    let ws_client = state.ws_client.read().await;
    ws_client.send(ClientMessage::AudioTrack { index }).await?;
    
    Ok(())
}

#[tauri::command]
async fn is_mpv_running(state: State<'_, AppState>) -> CommandResult<bool> {
    let mpv = state.mpv_manager.read().await;
    Ok(mpv.is_running().await)
}

// ============================================================================
// Helper Functions
// ============================================================================

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

// ============================================================================
// App Setup
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // Config
            set_config,
            get_config,
            // File Dialogs
            pick_file,
            pick_folder,
            save_file,
            // WebSocket
            connect,
            disconnect,
            is_connected,
            get_client_info,
            send_message,
            // Video Upload/Download
            upload_video,
            get_video_url,
            download_video,
            download_video_to_storage,
            check_video_downloaded,
            // MPV
            start_mpv,
            start_mpv_with_url,
            stop_mpv,
            mpv_pause,
            mpv_play,
            mpv_seek,
            mpv_set_subtitle_track,
            mpv_set_audio_track,
            is_mpv_running,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}