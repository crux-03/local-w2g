use tauri::{AppHandle, Manager, State};

use crate::{
    core::AppState,
    protocol::{ClientMessage, Permissions, Snowflake},
    CommandResult,
};

#[tauri::command]
pub async fn request_users(app: AppHandle) -> CommandResult<()> {
    app.state::<AppState>()
        .ws_send(ClientMessage::RequestUsers)
        .await
}

#[tauri::command]
pub async fn get_user_id(state: State<'_, AppState>) -> CommandResult<Snowflake> {
    Ok(state
        .client_id()
        .lock()
        .await
        .clone()
        .ok_or("Client id was not set".to_string())?)
}

#[tauri::command]
pub async fn update_permissions(
    state: State<'_, AppState>,
    target_user: Snowflake,
    permission: Permissions,
    granted: bool,
) -> CommandResult<()> {
    state
        .ws_send(ClientMessage::EditUserPermissions {
            target_user,
            permission,
            granted,
        })
        .await
}
