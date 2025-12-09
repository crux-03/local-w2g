use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade}, DefaultBodyLimit, Multipart, Path, Query, State
    },
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc, time};
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::Deserialize;
use chrono::Utc;

mod config;
mod state;
mod ws;
mod dto;

use config::Config;
use state::{AppState, VideoInfo};
use dto::ServerMessage;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::new("DEBUG")
        )
        .init();
    // Load config (will auto-create/fix config.yaml)
    let config = Config::load().expect("Failed to load config");
    tracing::info!("Server starting on {}:{}", config.host, config.port);

    // Initialize shared state
    let app_state = Arc::new(AppState::new(config.clone()));

    let max_upload_mb = config.max_file_size_mb * 1024 * 1024;

    // Build router
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/upload", post(upload_handler))
        .route("/video/{video_id}", get(download_handler))
        .layer(DefaultBodyLimit::max(max_upload_mb as usize))
        .route("/health", get(health_handler))
        .with_state(app_state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    tracing::info!("Server listening on {}", addr);
    tracing::info!("Video storage directory: {}", config.video_storage_dir);
    tracing::info!("Max file upload: {}MB", config.max_file_size_mb);
    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let client_id = Uuid::new_v4();

    // Create a channel for this client
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Add client to state with their sender
    state.add_client(client_id, tx).await;

    // Split socket into sender and receiver
    let (mut socket_tx, mut socket_rx) = socket.split();

    // Spawn task to forward messages from channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if socket_tx.send(msg).await.is_err() {
                tracing::error!("Failed to send message to client, disconnected");
                break;
            }
        }
    });

    // Send initial message
    if let Err(e) = ws::send_initial_message(client_id, Arc::clone(&state)).await {
        tracing::error!("Failed to send initial message: {}", e);
    }

    // Handle incoming messages
    while let Some(msg) = socket_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = ws::handle_command(&text, client_id, Arc::clone(&state)).await {
                    tracing::error!("Error handling command: {}", e);
                    let error_msg = ServerMessage::Error {
                        message: format!("Command error: {}", e)
                    };
                    if let Ok(bytes) = serde_json::to_string(&error_msg) {
                        state.send_to_client(&client_id, Message::Text(bytes.into())).await;
                    }
                }
            },
            Ok(Message::Close(_)) => break,
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            },
            _ => {}
        }
    }

    // Cleanup
    state.remove_client(&client_id).await;
    send_task.abort();

    // Send updated user list to remaining clients
    let _ = ws::send_user_update(Arc::clone(&state)).await;
}

#[derive(Debug, Deserialize)]
struct UploadQuery {
    client_id: Option<String>,
}

async fn upload_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UploadQuery>,
    mut multipart: Multipart,
) -> Result<Response, (StatusCode, String)> {
    tracing::info!("Uploading file...");
    // Parse client_id
    let client_id = if let Some(id_str) = query.client_id {
        Uuid::parse_str(&id_str)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid client_id: {}", e)))?
    } else {
        return Err((StatusCode::BAD_REQUEST, "Missing client_id parameter".to_string()));
    };

    // Check if client is owner
    if !state.is_owner(&client_id).await {
        return Err((StatusCode::FORBIDDEN, "Only the owner can upload videos".to_string()));
    }

    let mut filename: Option<String> = None;
    let mut file_size: u64 = 0;
    let video_id = Uuid::new_v4();

    let mut interval = time::interval(Duration::from_millis(2000));

    while let Some(mut field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let field_filename = field.file_name()
                .ok_or((StatusCode::BAD_REQUEST, "No filename provided".to_string()))?
                .to_string();

            filename = Some(field_filename.clone());

            // Validate file extension (basic video file check)
            let valid_extensions = [".mp4", ".mkv", ".avi", ".mov", ".webm", ".flv", ".wmv"];
            let is_valid = valid_extensions.iter().any(|ext| {
                field_filename.to_lowercase().ends_with(ext)
            });

            if !is_valid {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Invalid file type. Supported: mp4, mkv, avi, mov, webm, flv, wmv".to_string()
                ));
            }

            // Create file path with UUID to avoid conflicts
            let file_extension = std::path::Path::new(&field_filename)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("mp4");

            let file_path = format!(
                "{}/{}.{}",
                state.config.video_storage_dir,
                video_id,
                file_extension
            );

            // Stream file to disk
            let mut file = File::create(&file_path).await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create file: {}", e)))?;

            // Replace this single line:
            // let data = field.bytes().await...

            // With chunked reading:
            let mut bytes_received = 0u64;

            while let Some(chunk) = field.chunk().await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read chunk: {}", e)))?
            {
                bytes_received += chunk.len() as u64;

                // Check size limit as we go
                let max_size_bytes = state.config.max_file_size_mb * 1024 * 1024;
                if bytes_received > max_size_bytes {
                    let _ = tokio::fs::remove_file(&file_path).await;
                    return Err((
                        StatusCode::PAYLOAD_TOO_LARGE,
                        format!("File too large. Max size: {} MB", state.config.max_file_size_mb)
                    ));
                }

                // Write chunk to disk
                file.write_all(&chunk).await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write chunk: {}", e)))?;

                // Log progress every 2 seconds
                if interval.poll_tick(&mut std::task::Context::from_waker(futures_util::task::noop_waker_ref())).is_ready() {
                    let mb_received = bytes_received as f64 / (1024.0 * 1024.0);
                    tracing::debug!(
                        filename = %field_filename,
                        bytes_received_mb = mb_received,
                        "Upload progress"
                    );
                }
            }

            file_size = bytes_received;

            file.flush().await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to flush file: {}", e)))?;
        }
    }

    let filename = filename.ok_or((StatusCode::BAD_REQUEST, "No file uploaded".to_string()))?;

    // Create video info
    let video_info = VideoInfo {
        id: video_id,
        filename: filename.clone(),
        size_bytes: file_size,
        path: format!("{}/{}", state.config.video_storage_dir, video_id),
        uploaded_at: Utc::now(),
        uploader_id: client_id,
    };

    // Add to playlist
    state.add_video(video_info.clone()).await;

    // Notify all clients
    let video_dto = dto::Video {
        id: video_id.to_string(),
        filename: filename.clone(),
        size_bytes: file_size,
        size_display: dto::format_size(file_size),
        uploaded_at: video_info.uploaded_at,
        uploader_id: client_id.to_string(),
    };

    let message = ServerMessage::VideoUploaded { video: video_dto };
    let bytes = serde_json::to_string(&message)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Serialization error: {}", e)))?;

    state.broadcast_all(axum::extract::ws::Message::Text(bytes.into())).await;

    // Send updated playlist
    let _ = ws::send_playlist_update(Arc::clone(&state)).await;

    Ok((
        StatusCode::OK,
        format!("{{\"success\": true, \"video_id\": \"{}\", \"filename\": \"{}\", \"size\": {}}}",
            video_id, filename, file_size)
    ).into_response())
}

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    /// If true, force download. If false or not set, stream inline
    download: Option<bool>,
}

async fn download_handler(
    State(state): State<Arc<AppState>>,
    Path(video_id_str): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, (StatusCode, String)> {
    // Parse video ID
    let video_id = Uuid::parse_str(&video_id_str)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid video_id: {}", e)))?;

    // Find video in playlist
    let playlist = state.get_playlist().await;
    let video = playlist.iter().find(|v| v.id == video_id)
        .ok_or((StatusCode::NOT_FOUND, "Video not found".to_string()))?;

    // Determine file path
    let file_extension = std::path::Path::new(&video.filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("mp4");

    let file_path = format!(
        "{}/{}.{}",
        state.config.video_storage_dir,
        video_id,
        file_extension
    );

    // Check if file exists
    if !tokio::fs::metadata(&file_path).await.is_ok() {
        return Err((StatusCode::NOT_FOUND, "Video file not found on disk".to_string()));
    }

    // Open file
    let mut file = File::open(&file_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to open file: {}", e)))?;

    // Read file into memory (for simplicity; for large files consider streaming)
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read file: {}", e)))?;

    // Determine content type based on extension
    let content_type = match file_extension {
        "mp4" => "video/mp4",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "flv" => "video/x-flv",
        "wmv" => "video/x-ms-wmv",
        _ => "application/octet-stream",
    };

    // Build response
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, buffer.len());

    // Add Content-Disposition header
    if query.download.unwrap_or(false) {
        response = response.header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", video.filename)
        );
    } else {
        response = response.header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", video.filename)
        );
    }

    Ok(response
        .body(Body::from(buffer))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Response error: {}", e)))?)
}

async fn health_handler() -> impl IntoResponse {
    "OK"
}
