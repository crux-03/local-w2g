use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Pause,
    Play,
    Seek { position: f64 },
    SubtitleTrack { index: u32 },
    AudioTrack { index: u32 },
    Ready { value: bool },
    SetUsername { username: String },
    SetPermission { 
        client_id: String,
        permission: String,
        value: bool 
    },
    TransferOwnership { 
        client_id: String 
    },
    SelectVideo { 
        index: usize 
    },
    RequestState,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Sent when client first connects
    Connected {
        client_id: Uuid,
        is_owner: bool,
    },
    /// User list update
    UserUpdate {
        users: Vec<User>,
    },
    /// Permissions update
    PermissionsUpdate {
        permissions: Vec<UserPermission>,
    },
    /// Playlist update
    PlaylistUpdate {
        videos: Vec<Video>,
        current_index: Option<usize>,
    },
    /// Activity log update
    ActivityLog {
        logs: Vec<LogEntry>,
    },
    /// Playback control messages
    Pause,
    Play,
    Seek { position: f64 },
    SubtitleTrack { index: u32 },
    AudioTrack { index: u32 },
    Ready { client_id: Uuid, value: bool },
    VideoUploaded { 
        video: Video,
    },
    AllReady,
    OwnershipTransferred {
        new_owner_id: Uuid,
    },
    Error { message: String },
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: Option<String>,
    pub is_owner: bool,
    pub is_ready: bool,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Ready,
    Waiting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermission {
    pub user_id: String,
    pub allow_pause: bool,
    pub allow_seek: bool,
    pub allow_subtitle: bool,
    pub allow_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub id: String,
    pub filename: String,
    pub size_bytes: u64,
    pub size_display: String,
    pub uploaded_at: DateTime<Utc>,
    pub uploader_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub username: Option<String>,
    pub action: String,
    pub source: LogSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogSource {
    Server,
    Client,
}

/// Helper to format bytes into human-readable size
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}