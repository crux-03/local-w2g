use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    Json,
    body::Body,
    extract::{Query, State},
    http::StatusCode,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

use crate::Snowflake;
use crate::commands::Effect;
use crate::commands::handler::apply_effect;
use crate::core::AppState;
use crate::services::message::WidgetState;
use crate::services::permissions::Permissions;
use crate::services::videos::VIDEO_EXTENSIONS;
use crate::websocket::ServerMessage;

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Internal error: {e}"),
    )
}

/// Per-upload session. Lives in `AppState.upload_sessions` for the duration
/// of a chunked upload. Cleaned up by the reaper or by finalize.
pub struct UploadSession {
    pub client_id: Snowflake,
    pub filename: String,
    pub part_path: PathBuf,
    pub final_path: PathBuf,
    pub bytes_received: u64,
    pub bytes_expected: u64,
    pub widget_id: Snowflake, // adjust to whatever type `widget.id` is
    pub last_activity: Instant,
}

#[derive(Deserialize)]
pub struct InitQuery {
    client_id: Snowflake,
}

#[derive(Deserialize)]
pub struct UploadInitBody {
    filename: String,
    total_size: u64,
}

#[derive(Serialize)]
pub struct UploadInitResponse {
    upload_id: Snowflake,
}

#[derive(Deserialize)]
pub struct ChunkQuery {
    client_id: Snowflake,
    upload_id: Snowflake,
}

#[derive(Deserialize)]
pub struct FinalizeQuery {
    client_id: Snowflake,
    upload_id: Snowflake,
}

#[derive(Serialize)]
pub struct FinalizeResponse {
    video_id: Snowflake,
    filename: String,
    size_bytes: u64,
}

pub async fn upload_init_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<InitQuery>,
    Json(body): Json<UploadInitBody>,
) -> Result<Json<UploadInitResponse>, (StatusCode, String)> {
    state
        .services()
        .permission()
        .require(&q.client_id, Permissions::MANAGE_MEDIA)
        .await
        .map_err(|e| (StatusCode::FORBIDDEN, e.to_string()))?;

    let ext = std::path::Path::new(&body.filename)
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

    // Create empty .part so chunk handlers can open in append mode.
    File::create(&part_path).await.map_err(internal)?;

    let widget = state
        .services()
        .message()
        .create_widget(WidgetState::Upload {
            uploader: q.client_id,
            filename: body.filename.clone(),
            target: video_id,
            bytes_done: 0,
            bytes_total: body.total_size,
        })
        .await;
    let widget_id = widget.id;
    apply_effect(
        &state,
        Effect::Global(ServerMessage::MessageCreated { entry: widget }),
    )
    .await
    .map_err(internal)?;

    let session = UploadSession {
        client_id: q.client_id,
        filename: body.filename,
        part_path,
        final_path,
        bytes_received: 0,
        bytes_expected: body.total_size,
        widget_id,
        last_activity: Instant::now(),
    };
    state
        .upload_sessions()
        .write()
        .await
        .insert(video_id, session);

    Ok(Json(UploadInitResponse {
        upload_id: video_id,
    }))
}

pub async fn upload_chunk_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ChunkQuery>,
    body: Body,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .services()
        .permission()
        .require(&q.client_id, Permissions::MANAGE_MEDIA)
        .await
        .map_err(|e| (StatusCode::FORBIDDEN, e.to_string()))?;

    // Snapshot session fields without holding the lock during I/O.
    let (part_path, filename, widget_id, bytes_expected) = {
        let sessions = state.upload_sessions().read().await;
        let s = sessions
            .get(&q.upload_id)
            .ok_or((StatusCode::NOT_FOUND, "Upload session not found".into()))?;
        if s.client_id != q.client_id {
            return Err((
                StatusCode::FORBIDDEN,
                "Upload belongs to a different client".into(),
            ));
        }
        (
            s.part_path.clone(),
            s.filename.clone(),
            s.widget_id,
            s.bytes_expected,
        )
    };

    let mut file = OpenOptions::new()
        .append(true)
        .open(&part_path)
        .await
        .map_err(internal)?;

    let mut stream = body.into_data_stream();
    let mut chunk_bytes: u64 = 0;
    while let Some(next) = stream.next().await {
        let buf = next.map_err(|e| (StatusCode::BAD_REQUEST, format!("Body read error: {e}")))?;
        file.write_all(&buf).await.map_err(internal)?;
        chunk_bytes += buf.len() as u64;
    }
    file.flush().await.map_err(internal)?;

    let new_total = {
        let mut sessions = state.upload_sessions().write().await;
        let s = sessions.get_mut(&q.upload_id).ok_or((
            StatusCode::NOT_FOUND,
            "Upload session disappeared mid-chunk".into(),
        ))?;
        s.bytes_received += chunk_bytes;
        s.last_activity = Instant::now();
        s.bytes_received
    };

    let updated = state
        .services()
        .message()
        .update_widget(
            widget_id,
            WidgetState::Upload {
                uploader: q.client_id,
                filename,
                target: q.upload_id,
                bytes_done: new_total,
                bytes_total: bytes_expected.max(new_total),
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

    Ok(StatusCode::OK)
}

pub async fn upload_finalize_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<FinalizeQuery>,
) -> Result<Json<FinalizeResponse>, (StatusCode, String)> {
    state
        .services()
        .permission()
        .require(&q.client_id, Permissions::MANAGE_MEDIA)
        .await
        .map_err(|e| (StatusCode::FORBIDDEN, e.to_string()))?;

    let session = {
        let mut sessions = state.upload_sessions().write().await;
        match sessions.get(&q.upload_id) {
            None => return Err((StatusCode::NOT_FOUND, "Upload session not found".into())),
            Some(s) if s.client_id != q.client_id => {
                return Err((
                    StatusCode::FORBIDDEN,
                    "Upload belongs to a different client".into(),
                ));
            }
            Some(_) => sessions.remove(&q.upload_id).unwrap(),
        }
    };

    tokio::fs::rename(&session.part_path, &session.final_path)
        .await
        .map_err(internal)?;

    state
        .services()
        .video()
        .register(q.upload_id, session.filename.clone())
        .await
        .map_err(internal)?;

    let final_widget = state
        .services()
        .message()
        .finish_widget(
            session.widget_id,
            WidgetState::Upload {
                uploader: q.client_id,
                filename: session.filename.clone(),
                target: q.client_id,
                bytes_done: session.bytes_received,
                bytes_total: session.bytes_received,
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

    let playlist = state.services().video().get_playlist().await;
    apply_effect(
        &state,
        Effect::Global(ServerMessage::PlaylistUpdated { playlist }),
    )
    .await
    .map_err(internal)?;

    Ok(Json(FinalizeResponse {
        video_id: q.upload_id,
        filename: session.filename,
        size_bytes: session.bytes_received,
    }))
}

/// Background task: removes sessions idle for >30 min, deleting the .part
/// file and closing the widget. Spawn once at startup.
pub async fn upload_reaper(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    let ttl = Duration::from_secs(30 * 60);
    loop {
        interval.tick().await;
        let now = Instant::now();

        let stale: Vec<(Snowflake, UploadSession)> = {
            let mut sessions = state.upload_sessions().write().await;
            let ids: Vec<Snowflake> = sessions
                .iter()
                .filter(|(_, s)| now.duration_since(s.last_activity) > ttl)
                .map(|(id, _)| *id)
                .collect();
            ids.into_iter()
                .filter_map(|id| sessions.remove(&id).map(|s| (id, s)))
                .collect()
        };

        for (id, session) in stale {
            let _ = tokio::fs::remove_file(&session.part_path).await;
            if let Ok(w) = state
                .services()
                .message()
                .finish_widget(
                    session.widget_id,
                    WidgetState::Upload {
                        uploader: Snowflake::system(),
                        filename: session.filename,
                        target: id,
                        bytes_done: session.bytes_received,
                        bytes_total: session.bytes_expected.max(session.bytes_received),
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
        }
    }
}
