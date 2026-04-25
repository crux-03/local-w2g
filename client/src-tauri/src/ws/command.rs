use tauri::{AppHandle, Manager};

use crate::core::AppState;

fn format_url(base_url: String) -> String {
    if base_url.starts_with("ws://") || base_url.starts_with("wss://") {
        base_url.to_string()
    } else {
        let base = base_url.trim_end_matches('/');
        if base_url.starts_with("https://") {
            format!("{}/ws", base.replace("https://", "wss://"))
        } else if base_url.starts_with("http://") {
            format!("{}/ws", base.replace("http://", "ws://"))
        } else {
            format!("ws://{}/ws", base)
        }
    }
}

#[tauri::command]
pub async fn connect(
    username: String,
    server_url: String,
    server_pw: String,
    app: AppHandle,
) -> crate::CommandResult<()> {
    let app_clone = app.clone();
    let state = app.state::<AppState>();
    state.set_server_url(&app, server_url.clone()).await?;
    state.set_username(&app, username.clone()).await?;

    let ws_url = format_url(server_url);
    let handle = super::spawn(ws_url, username, server_pw, app_clone);
    state.set_ws_handle(handle).await;

    tracing::info!("WebSocket connected");
    
    state
        .ws_send(crate::protocol::ClientMessage::RequestIdentity)
        .await?;
    tracing::info!("Identity retrieved");

    Ok(())
}
