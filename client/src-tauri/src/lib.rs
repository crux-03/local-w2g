mod mpv;
mod types;
mod websocket;

use chrono::Utc;
use futures_util::StreamExt;
use mpv::MpvManager;
use reqwest::Body;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tokio_util::io::ReaderStream;
use types::{ClientMessage, CommandResult, LogEntry, LogSource, Video};
use websocket::WebSocketClient;

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
    password: Arc<RwLock<String>>,
    config_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ClientConfig {
    server_url: Option<String>,
    video_storage_path: Option<String>,
    mpv_binary_path: Option<String>,
}

impl AppState {
    fn new(app_handle: &AppHandle) -> Self {
        // Get the app config directory
        let config_dir = app_handle
            .path()
            .app_config_dir()
            .expect("Failed to get config directory");
        
        // Create the config directory if it doesn't exist
        std::fs::create_dir_all(&config_dir).ok();
        
        let config_path = config_dir.join("config.json");
        
        // Load config from file if it exists
        let config = if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => {
                    serde_json::from_str(&content).unwrap_or_else(|_| ClientConfig {
                        server_url: None,
                        video_storage_path: None,
                        mpv_binary_path: None,
                    })
                }
                Err(_) => ClientConfig {
                    server_url: None,
                    video_storage_path: None,
                    mpv_binary_path: None,
                },
            }
        } else {
            ClientConfig {
                server_url: None,
                video_storage_path: None,
                mpv_binary_path: None,
            }
        };

        Self {
            ws_client: Arc::new(RwLock::new(WebSocketClient::new())),
            mpv_manager: Arc::new(RwLock::new(MpvManager::new())),
            config: Arc::new(RwLock::new(config)),
            client_id: Arc::new(RwLock::new(None)),
            is_owner: Arc::new(RwLock::new(false)),
            password: Arc::new(RwLock::new(String::new())),
            config_path,
        }
    }
    
    async fn save_config(&self) -> Result<(), String> {
        let config = self.config.read().await;
        let json = serde_json::to_string_pretty(&*config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        std::fs::write(&self.config_path, json)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        
        Ok(())
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
    
    drop(config); // Release the lock before saving
    state.save_config().await?;

    Ok(())
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> CommandResult<ClientConfig> {
    let config = state.config.read().await;
    Ok(config.clone())
}

#[tauri::command]
async fn set_password(password: String, state: State<'_, AppState>) -> CommandResult<()> {
    let mut pw = state.password.write().await;
    *pw = password;
    Ok(())
}

// ============================================================================
// File Dialog Commands
// ============================================================================

#[tauri::command]
async fn pick_file(filters: Option<Vec<(String, Vec<String>)>>) -> CommandResult<Option<String>> {
    let task = tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();

        // Add filters if provided
        if let Some(filter_list) = filters {
            for (name, extensions) in filter_list {
                dialog = dialog.add_filter(&name, &extensions);
            }
        }

        dialog
            .pick_file()
            .map(|path| path.to_string_lossy().to_string())
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
async fn save_file(default_name: Option<String>) -> CommandResult<Option<String>> {
    let task = tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();

        if let Some(name) = default_name {
            dialog = dialog.set_file_name(&name);
        }

        dialog
            .save_file()
            .map(|path| path.to_string_lossy().to_string())
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
    server_pw: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> CommandResult<()> {
    emit_client_log(&app_handle, format!("Connecting to {}", server_url));

    // Update config
    state.config.write().await.server_url = Some(server_url.clone());
    state.save_config().await?;

    // Connect to WebSocket
    let mut ws_client = state.ws_client.write().await;

    // Pass state references for storing client_id and is_owner
    
    ws_client.connect(
        &server_url,
        app_handle.clone(),
        state.client_id.clone(),
        state.is_owner.clone(),
        server_pw.clone()
    ).await?;

    emit_client_log(&app_handle, "Connected successfully".to_string());
    
    let mut password = state.password.write().await;
    *password = server_pw;
    
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
            let client_id = obj
                .get("client_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or("Missing client_id")?;
            let permission = obj
                .get("permission")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or("Missing permission")?;
            let value = obj
                .get("value")
                .and_then(|v| v.as_bool())
                .ok_or("Missing value")?;
            ClientMessage::SetPermission {
                client_id,
                permission,
                value,
            }
        }
        "transfer_ownership" => {
            let client_id = data
                .and_then(|d| d.as_str().map(|s| s.to_string()))
                .ok_or("Missing client_id")?;
            ClientMessage::TransferOwnership { client_id }
        }
        "select_video" => {
            let index = data
                .and_then(|d| d.as_u64())
                .map(|i| i as usize)
                .ok_or("Missing or invalid index")?;
            ClientMessage::SelectVideo { index }
        }
        "request_state" => ClientMessage::RequestState,
        "ping" => ClientMessage::Ping,
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
) -> CommandResult<()> {
    emit_client_log(&app_handle, format!("Uploading video: {}", file_path));

    let config = state.config.read().await;
    let server_url = config
        .server_url
        .as_ref()
        .ok_or("Server URL not configured")?
        .clone();
    drop(config);

    let client_id = state
        .client_id
        .read()
        .await
        .as_ref()
        .ok_or("Not connected")?
        .clone();

    let password = state.password.read().await.clone();

    // Open file
    let file = tokio::fs::File::open(&file_path)
        .await
        .map_err(|e| format!("Failed to open file: {}", e))?;

    let metadata = file
        .metadata()
        .await
        .map_err(|e| format!("Failed to get file metadata: {}", e))?;
    let file_size = metadata.len();

    emit_client_log(
        &app_handle,
        format!("File size: {}", format_size(file_size)),
    );

    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?
        .to_string();

    // Create multipart form
    let stream = ReaderStream::new(file);
    let part = reqwest::multipart::Part::stream(Body::wrap_stream(stream))
        .file_name(filename.clone())
        .mime_str("video/mp4")
        .map_err(|e| format!("Failed to create multipart: {}", e))?;

    let form = reqwest::multipart::Form::new().part("file", part);

    let server_url = if server_url.starts_with("http://") || server_url.starts_with("https://") {
        server_url
    } else {
        format!("http://{}", server_url)
    };
    let url = format!("{}/upload?client_id={}", server_url, client_id);

    emit_client_log(&app_handle, "Starting upload...".to_string());

    // Send request
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("X-Access-Key", password)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Upload request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Upload failed ({}): {}", status, error_text));
    }

    emit_client_log(&app_handle, "Video uploaded successfully".to_string());

    Ok(())
}

#[tauri::command]
async fn get_video_url(
    video_id: String,
    download: Option<bool>,
    state: State<'_, AppState>,
) -> CommandResult<String> {
    let config = state.config.read().await;
    let server_url = config
        .server_url
        .as_ref()
        .ok_or("Server URL not configured")?;
    
    let server_url = if server_url.starts_with("http://") || server_url.starts_with("https://") {
        server_url
    } else {
        &format!("http://{}", server_url)
    };

    let download_param = if download.unwrap_or(false) {
        "?download=true"
    } else {
        ""
    };

    Ok(format!(
        "{}/video/{}{}",
        server_url, video_id, download_param
    ))
}

#[tauri::command]
async fn download_video(
    video_id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> CommandResult<Vec<u8>> {
    emit_client_log(&app_handle, format!("Downloading video: {}", video_id));

    let video_url = get_video_url(video_id, Some(true), state.clone()).await?;
    let password = state.password.read().await.clone();

    let client = reqwest::Client::new();
    let response = client
        .get(&video_url)
        .header("X-Access-Key", password)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    emit_client_log(&app_handle, "Video downloaded successfully".to_string());

    Ok(bytes.to_vec())
}

#[tauri::command]
async fn download_video_to_storage(
    video_id: String,
    filename: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> CommandResult<String> {
    emit_client_log(
        &app_handle,
        format!("Downloading video to storage: {}", filename),
    );

    // Get config
    let config = state.config.read().await;
    let storage_path = config
        .video_storage_path
        .as_ref()
        .ok_or("Video storage path not configured")?
        .clone();
    let mut server_url = config
        .server_url
        .as_ref()
        .ok_or("Server URL not configured")?
        .clone();
    drop(config);

    // Ensure server URL has http:// or https:// prefix
    if !server_url.starts_with("http://") && !server_url.starts_with("https://") {
        server_url = format!("http://{}", server_url);
    }

    // Create storage directory if it doesn't exist
    tokio::fs::create_dir_all(&storage_path)
        .await
        .map_err(|e| format!("Failed to create storage directory: {}", e))?;

    let video_path = std::path::Path::new(&storage_path).join(&filename);
    let video_path_str = video_path.to_string_lossy().to_string();

    // Prepare URL
    let url = format!("{}/video/{}", server_url, video_id);
    let password = state.password.read().await.clone();

    // Clone state for the spawned task
    let ws_client = state.ws_client.clone();
    let app_handle_clone = app_handle.clone();
    let video_id_clone = video_id.clone();
    let filename_clone = filename.clone();
    
    tokio::spawn(async move {
        // Create file
        let mut file = match tokio::fs::File::create(&video_path).await {
            Ok(f) => f,
            Err(e) => {
                emit_client_log(&app_handle_clone, format!("Failed to create file: {}", e));
                return;
            }
        };

        // Start download
        let client = reqwest::Client::new();
        let response = match client
            .get(&url)
            .header("X-Access-Key", password)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                emit_client_log(&app_handle_clone, format!("Download request failed: {}", e));
                // Clean up empty file
                let _ = tokio::fs::remove_file(&video_path).await;
                return;
            }
        };

        if !response.status().is_success() {
            emit_client_log(
                &app_handle_clone,
                format!("Download failed with status: {}", response.status()),
            );
            // Clean up empty file
            let _ = tokio::fs::remove_file(&video_path).await;
            return;
        }

        let total_size = response.content_length().unwrap_or(0);

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut last_update = std::time::Instant::now();
        let mut last_downloaded: u64 = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    emit_client_log(&app_handle_clone, format!("Failed to read chunk: {}", e));
                    // Clean up partial file
                    let _ = tokio::fs::remove_file(&video_path).await;
                    return;
                }
            };

            if let Err(e) = file.write_all(&chunk).await {
                emit_client_log(&app_handle_clone, format!("Failed to write chunk: {}", e));
                // Clean up partial file
                let _ = tokio::fs::remove_file(&video_path).await;
                return;
            }

            downloaded += chunk.len() as u64;

            // Update progress every 500ms
            let now = std::time::Instant::now();
            if now.duration_since(last_update) >= Duration::from_millis(500) {
                let progress = if total_size > 0 {
                    (downloaded as f64 / total_size as f64 * 100.0) as u64
                } else {
                    0
                };

                let elapsed_secs = now.duration_since(last_update).as_secs_f64();
                let bytes_since_last = downloaded - last_downloaded;
                let speed = if elapsed_secs > 0.0 {
                    (bytes_since_last as f64 / elapsed_secs) as u64
                } else {
                    0
                };

                let speed_display = format_speed(speed);

                // Emit local progress event for UI responsiveness
                let progress_data = serde_json::json!({
                    "video_id": video_id_clone,
                    "filename": filename_clone,
                    "downloaded": downloaded,
                    "total": total_size,
                    "progress": progress,
                    "speed": speed,
                    "speed_display": speed_display,
                });
                let _ = app_handle_clone.emit("download-progress", progress_data);

                // Send progress update to server (to broadcast to all clients)
                let ws = ws_client.read().await;
                let _ = ws.send(ClientMessage::DownloadProgress {
                    video_id: video_id_clone.clone(),
                    filename: filename_clone.clone(),
                    downloaded,
                    total: total_size,
                    progress,
                    speed,
                    speed_display: speed_display.clone(),
                }).await;
                drop(ws);

                last_update = now;
                last_downloaded = downloaded;
            }
        }

        if let Err(e) = file.flush().await {
            emit_client_log(&app_handle_clone, format!("Failed to flush file: {}", e));
            // Clean up incomplete file
            let _ = tokio::fs::remove_file(&video_path).await;
            return;
        }

        emit_client_log(&app_handle_clone, "Download complete".to_string());

        // Emit final progress (100%)
        let progress_data = serde_json::json!({
            "video_id": video_id_clone,
            "filename": filename_clone,
            "downloaded": downloaded,
            "total": total_size,
            "progress": 100,
            "speed": 0,
            "speed_display": "Complete".to_string(),
        });
        let _ = app_handle_clone.emit("download-progress", progress_data);

        // Send final progress to server
        let ws = ws_client.read().await;
        let _ = ws.send(ClientMessage::DownloadProgress {
            video_id: video_id_clone,
            filename: filename_clone,
            downloaded,
            total: total_size,
            progress: 100,
            speed: 0,
            speed_display: "Complete".to_string(),
        }).await;
    });

    // Return immediately
    Ok(video_path_str)
}

// Helper function to format speed
fn format_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        return "0 B/s".to_string();
    }

    let kb = bytes_per_sec as f64 / 1024.0;
    if kb < 1024.0 {
        return format!("{:.1} KB/s", kb);
    }

    let mb = kb / 1024.0;
    if mb < 1024.0 {
        return format!("{:.1} MB/s", mb);
    }

    let gb = mb / 1024.0;
    format!("{:.1} GB/s", gb)
}

#[tauri::command]
async fn check_video_downloaded(
    filename: String,
    state: State<'_, AppState>,
) -> CommandResult<Option<String>> {
    let config = state.config.read().await;
    let storage_path = config
        .video_storage_path
        .as_ref()
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

    emit_client_log(
        &app_handle,
        format!("Starting MPV with local file: {}", video_path),
    );

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

    emit_client_log(
        &app_handle,
        format!("Starting MPV with stream: {}", video_url),
    );

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
async fn mpv_seek_relative(offset: f64, state: State<'_, AppState>) -> CommandResult<()> {
    let mpv = state.mpv_manager.read().await;
    
    // For relative seeking, we can send a relative seek command
    // MPV supports relative seeking with the "relative" flag
    mpv.send_command(vec![
        serde_json::json!("seek"),
        serde_json::json!(offset),
        serde_json::json!("relative")
    ]).await?;

    Ok(())
}

#[tauri::command]
async fn mpv_get_time_pos(state: State<'_, AppState>) -> CommandResult<f64> {
    let mpv = state.mpv_manager.read().await;
    
    // Note: This is a simplified implementation
    // For proper property getting, you'd need to implement response parsing
    // For now, we'll return 0 as a placeholder
    // You would need to implement proper IPC response handling in mpv.rs
    Ok(0.0)
}

#[tauri::command]
async fn mpv_set_subtitle_track(index: u32, state: State<'_, AppState>) -> CommandResult<()> {
    let mpv = state.mpv_manager.read().await;
    mpv.set_subtitle_track(index).await?;

    // Also send to server
    let ws_client = state.ws_client.read().await;
    ws_client
        .send(ClientMessage::SubtitleTrack { index })
        .await?;

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
        .setup(|app| {
            let state = AppState::new(app.handle());
            app.manage(state);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // Config
            set_config,
            get_config,
            set_password,
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
            mpv_seek_relative,
            mpv_get_time_pos,
            mpv_set_subtitle_track,
            mpv_set_audio_track,
            is_mpv_running,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}