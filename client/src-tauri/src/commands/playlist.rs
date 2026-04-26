use tauri::State;

use crate::{
    core::AppState,
    protocol::{ClientMessage, Snowflake},
    CommandResult,
};

#[tauri::command]
pub async fn request_playlist(state: State<'_, AppState>) -> CommandResult<()> {
    state.ws_send(ClientMessage::RequestPlaylist).await
}

#[tauri::command]
pub async fn swap_entries(
    state: State<'_, AppState>,
    first: Snowflake,
    second: Snowflake,
) -> CommandResult<()> {
    state
        .ws_send(ClientMessage::SwapEntries { first, second })
        .await
}

#[tauri::command]
pub async fn update_entry_display_name(
    state: State<'_, AppState>,
    id: Snowflake,
    display_name: String,
) -> CommandResult<()> {
    state
        .ws_send(ClientMessage::SetDisplayName {
            video_id: id,
            display_name,
        })
        .await
}

#[tauri::command]
pub async fn update_entry_audio_track(
    state: State<'_, AppState>,
    id: Snowflake,
    audio_track: i32,
) -> CommandResult<()> {
    state
        .ws_send(ClientMessage::SetAudioTrack {
            video_id: id,
            audio_track,
        })
        .await
}

#[tauri::command]
pub async fn update_entry_subtitle_track(
    state: State<'_, AppState>,
    id: Snowflake,
    subtitle_track: i32,
) -> CommandResult<()> {
    state
        .ws_send(ClientMessage::SetSubtitleTrack {
            video_id: id,
            subtitle_track,
        })
        .await
}
