use std::sync::Arc;

use axum::{
    http::StatusCode,
    middleware::Next,
    response::Response,
    extract::State,
};

use crate::state::AppState;

pub async fn require_password(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(provided) = req.headers().get("X-Access-Key") else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let provided = provided.to_str().unwrap_or("");

    if provided != state.access_password {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}
