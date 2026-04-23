use std::sync::Arc;

use axum::{Router, routing::{get, post}};

use crate::core::AppState;

mod upload;
mod download;

pub fn api(state: Arc<AppState>) -> Router {
    Router::new()
        .route("upload", post(upload::upload_handler))
        .route("videos/{video_id}", get(download::download_handler))
        .with_state(state)
}