use tauri::Manager;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::core::AppState;

mod commands;
mod core;
mod error;
mod mpv;
mod protocol;
mod ws;

pub type CommandResult<T> = Result<T, String>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::new("DEBUG"))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            tracing::info!("Setting up AppState");
            app.manage(AppState::new(app.handle()).expect("AppState should build"));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::load_username,
            commands::config::load_server_url,
            commands::config::load_mpv_binary,
            commands::config::load_videos_dir,
            commands::config::set_mpv_binary,
            commands::config::set_videos_dir,
            commands::config::password_to_clipboard,
            commands::file::init_file_manager,
            commands::file::file_on_device,
            commands::file::load_local_files,
            commands::file::download_file,
            commands::io::pick_file,
            commands::io::pick_folder,
            commands::user::get_user_id,
            commands::user::request_users,
            commands::user::update_permissions,
            commands::messages::request_message_history,
            commands::messages::send_chat_message,
            commands::playback::init_mpv_manager,
            commands::playback::play,
            commands::playback::resume,
            commands::playback::pause,
            commands::playback::seek,
            commands::playback::select_video,
            commands::playback::resync,
            commands::playlist::request_playlist,
            commands::playlist::swap_entries,
            commands::playlist::update_entry_display_name,
            commands::playlist::update_entry_audio_track,
            commands::playlist::update_entry_subtitle_track,
            commands::upload::upload_video,
            ws::command::connect,
            core::probe_media,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
