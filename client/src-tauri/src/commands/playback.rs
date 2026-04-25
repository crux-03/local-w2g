use tauri::State;

use crate::protocol::{ClientMessage, Snowflake};
use crate::ws::WsHandle;

#[tauri::command]
pub async fn play(ws: State<'_, WsHandle>) -> Result<(), String> {
    ws.send(ClientMessage::Play).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn select_video(video_id: Snowflake, ws: State<'_, WsHandle>) -> Result<(), String> {
    ws.send(ClientMessage::SelectVideo { video_id })
        .map_err(|e| e.to_string())
}
