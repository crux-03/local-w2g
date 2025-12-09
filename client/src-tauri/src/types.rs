use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Messages sent from client to server
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

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Sent when client first connects
    Connected {
        client_id: String,
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
    Ready { client_id: String, value: bool },
    VideoUploaded { 
        video: Video,
    },
    AllReady,
    OwnershipTransferred {
        new_owner_id: String,
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

/// Tauri command results
pub type CommandResult<T> = Result<T, String>;

/// Upload progress callback data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadProgress {
    pub loaded: u64,
    pub total: u64,
    pub percentage: f64,
}