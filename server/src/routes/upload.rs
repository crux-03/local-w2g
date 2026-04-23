use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    Json,
    extract::{Multipart, Query, State},
    http::{HeaderMap, StatusCode, header::CONTENT_LENGTH},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::commands::Effect;
use crate::commands::handler::apply_effect;
use crate::services::permissions::Permissions;
use crate::services::videos::{PartialFileGuard, VIDEO_EXTENSIONS};
use crate::websocket::ServerMessage;
use crate::{Snowflake, core::AppState, services::message::WidgetState};

#[derive(Deserialize)]
pub struct UploadQuery {
    client_id: Snowflake,
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Internal error: {e}"),
    )
}

pub async fn upload_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UploadQuery>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Response, (StatusCode, String)> {
    state
        .services()
        .permission()
        .require(&query.client_id, Permissions::MANAGE_PLAYLIST)
        .await
        .map_err(|e| (StatusCode::FORBIDDEN, e.to_string()))?;

    let total_estimate = headers
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Find the file field; skip anything else the client sends along.
    let mut field = loop {
        match multipart
            .next_field()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {e}")))?
        {
            Some(f) if f.name() == Some("file") => break f,
            Some(_) => continue,
            None => return Err((StatusCode::BAD_REQUEST, "No file field in request".into())),
        }
    };

    let filename = field
        .file_name()
        .ok_or((StatusCode::BAD_REQUEST, "No filename provided".into()))?
        .to_string();

    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .ok_or((StatusCode::BAD_REQUEST, "File has no extension".into()))?;

    if !VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Unsupported extension: .{ext}. Allowed: {VIDEO_EXTENSIONS:?}"),
        ));
    }

    let (video_id, final_path) = state.services().video().reserve(&ext);
    let part_path: PathBuf = final_path.with_extension(format!("{ext}.part"));
    let guard = PartialFileGuard::new(part_path.clone());

    // Create and broadcast the initial widget.
    let widget = state
        .services()
        .message()
        .create_widget(WidgetState::Upload {
            filename: filename.clone(),
            bytes_done: 0,
            bytes_total: total_estimate,
        })
        .await;
    let widget_id = widget.id;
    apply_effect(
        &state,
        Effect::Global(ServerMessage::WidgetUpdated { entry: widget }),
    )
    .await
    .map_err(internal)?;

    // Stream to disk with throttled progress broadcasts. `bytes_done` is
    // kept outside the async block so the error path can report the real
    // number instead of 0.
    let mut bytes_done: u64 = 0;
    let stream_result: Result<(), (StatusCode, String)> = async {
        let mut file = File::create(&part_path).await.map_err(internal)?;
        let mut last_update = Instant::now();
        let update_interval = Duration::from_millis(500);

        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Chunk read error: {e}")))?
        {
            bytes_done += chunk.len() as u64;
            file.write_all(&chunk).await.map_err(internal)?;

            if last_update.elapsed() >= update_interval {
                last_update = Instant::now();
                let updated = state
                    .services()
                    .message()
                    .update_widget(
                        widget_id,
                        WidgetState::Upload {
                            filename: filename.clone(),
                            bytes_done,
                            bytes_total: total_estimate.max(bytes_done),
                        },
                    )
                    .await
                    .map_err(internal)?;
                apply_effect(
                    &state,
                    Effect::Global(ServerMessage::WidgetUpdated { entry: updated }),
                )
                .await
                .map_err(internal)?;
            }
        }

        file.flush().await.map_err(internal)?;
        Ok(())
    }
    .await;

    if let Err(e) = stream_result {
        // Best-effort: finalize the widget so clients don't see a stuck
        // progress bar. If this fails, the original error still propagates.
        if let Ok(w) = state
            .services()
            .message()
            .finish_widget(
                widget_id,
                WidgetState::Upload {
                    filename: filename.clone(),
                    bytes_done,
                    bytes_total: total_estimate.max(bytes_done),
                },
            )
            .await
        {
            let _ = apply_effect(
                &state,
                Effect::Global(ServerMessage::WidgetDone { entry: w }),
            )
            .await;
        }
        // guard drops → .part file removed
        return Err(e);
    }

    // Success path: atomic rename, register, finalize widget.
    tokio::fs::rename(&part_path, &final_path)
        .await
        .map_err(internal)?;
    guard.commit();

    state
        .services()
        .video()
        .register(video_id, filename.clone())
        .await
        .map_err(internal)?;

    let final_widget = state
        .services()
        .message()
        .finish_widget(
            widget_id,
            WidgetState::Upload {
                filename: filename.clone(),
                bytes_done,
                bytes_total: bytes_done,
            },
        )
        .await
        .map_err(internal)?;
    apply_effect(
        &state,
        Effect::Global(ServerMessage::WidgetDone {
            entry: final_widget,
        }),
    )
    .await
    .map_err(internal)?;

    Ok(Json(json!({
        "video_id": video_id,
        "filename": filename,
        "size_bytes": bytes_done,
    }))
    .into_response())
}
