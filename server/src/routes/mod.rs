use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use crate::core::AppState;

mod download;
mod upload;

pub use upload::UploadSession;

pub fn api(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/upload/init", post(upload::upload_init_handler))
        .route("/upload/chunk", post(upload::upload_chunk_handler))
        .route("/upload/finalize", post(upload::upload_finalize_handler))
        .route("/videos/{video_id}", get(download::download_handler))
        .with_state(state)
}
