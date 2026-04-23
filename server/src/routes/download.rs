use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Response,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio_util::io::ReaderStream;

use crate::{Snowflake, commands::{Effect, handler::apply_effect}, core::AppState, services::message::WidgetState, websocket::ServerMessage};

#[derive(Deserialize)]
pub struct DownloadQuery {
    client_id: Snowflake,
}

pub async fn download_handler(
    State(state): State<Arc<AppState>>,
    Path(video_id): Path<Snowflake>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, (StatusCode, String)> {
    let path = state
        .services()
        .video()
        .resolve_path(video_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Video not found".into()))?;
    let display_name = state
        .services()
        .video()
        .display_name(video_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Video not found".into()))?;

    let file = tokio::fs::File::open(&path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Open failed: {e}"),
        )
    })?;
    let size = file
        .metadata()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Stat failed: {e}"),
            )
        })?
        .len();

    // Create widget and broadcast initial state before streaming.
    let widget = state
        .services()
        .message()
        .create_widget(WidgetState::Download {
            reporter: query.client_id,
            filename: display_name.clone(),
            bytes_done: 0,
            bytes_total: size,
        })
        .await;
    let widget_id = widget.id;

    apply_effect(
        &state,
        Effect::Global(ServerMessage::WidgetUpdated { entry: widget }),
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Broadcast failed: {e}"),
        )
    })?;

    let content_type = match path.extension().and_then(|e| e.to_str()) {
        Some("mp4") => "video/mp4",
        Some("mkv") => "video/x-matroska",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        _ => "application/octet-stream",
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, size)
        .header("X-Widget-Id", widget_id.to_string())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{display_name}\""),
        )
        .body(Body::from_stream(ReaderStream::new(file)))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Response build failed: {e}"),
            )
        })
}
