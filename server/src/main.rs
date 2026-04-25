use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{Router, routing::get};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod core;
mod error;
mod routes;
mod services;
mod types;
mod websocket;

pub use error::Error;
pub use types::*;

use crate::{
    commands::{
        Effect,
        handler::{apply_effect, execute_command},
        messages::UpdateWidgetCommand,
    },
    core::AppState,
    services::message::{EntryKind, WidgetState},
    websocket::ServerMessage,
};

async fn start_widget_demo(state: Arc<AppState>) -> anyhow::Result<()> {
    let widget = state
        .services()
        .message()
        .create_widget(WidgetState::Upload {
            uploader: Snowflake::system(),
            filename: "test.mp4".to_string(),
            target: Snowflake(-1),
            bytes_done: 0,
            bytes_total: 10 * 1024 * 1024,
        })
        .await;

    let mut widget_state = match widget.kind {
        EntryKind::Widget { state, done: _ } => state,
        _ => {
            return Err(anyhow::anyhow!("This should never happen"));
        }
    };

    let initial_command = UpdateWidgetCommand {
        msg_id: widget.id,
        state: widget_state.clone(),
        finished: false,
    };

    execute_command(Box::new(initial_command), Snowflake(0), Arc::clone(&state)).await?;

    const CHUNK: u64 = 1024 * 1024; // 1 MiB per 500ms = ~10 ticks to finish

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;

        let finished = match &mut widget_state {
            WidgetState::Upload {
                bytes_done,
                bytes_total,
                ..
            } => {
                *bytes_done = (*bytes_done + CHUNK).min(*bytes_total);
                *bytes_done >= *bytes_total
            }
            _ => continue,
        };

        let command = UpdateWidgetCommand {
            msg_id: widget.id,
            state: widget_state.clone(),
            finished,
        };
        execute_command(Box::new(command), Snowflake(0), Arc::clone(&state)).await?;

        if finished {
            break;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::new("DEBUG"))
        .init();

    let app_state = Arc::new(AppState::new().await?);

    let sweep_state = Arc::clone(&app_state);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        loop {
            ticker.tick().await;
            let stale = sweep_state.services().state().sweep_stale().await;
            for user_id in stale {
                let view = sweep_state.services().state().view_for(user_id).await;
                let _ = apply_effect(
                    &sweep_state,
                    Effect::Global(ServerMessage::ReadinessUpdated { readiness: view }),
                )
                .await;
            }
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let ws_router = Router::new()
        .route("/ws", get(websocket::websocket_handler))
        //.layer(...) TODO: implement password auth
        .with_state(Arc::clone(&app_state));

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .merge(ws_router)
        .nest("/api/v1", routes::api(Arc::clone(&app_state)))
        .layer(cors);

    tokio::spawn(async move { start_widget_demo(Arc::clone(&app_state)).await });

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
