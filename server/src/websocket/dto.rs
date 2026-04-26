use serde::{Deserialize, Serialize};

use crate::{
    Snowflake,
    services::{
        message::Entry, permissions::Permissions, state::UserReadinessView, user::User,
        videos::VideoEntry,
    },
};

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    // Utility
    Pong,

    // Users
    UserIdentity {
        id: Snowflake,
    },
    UserList {
        users: Vec<User>,
    },
    PermissionUpdate {
        user_id: Snowflake,
        permissions: Permissions,
    },
    // Messages
    MessageCreated {
        entry: Entry,
    },
    WidgetUpdated {
        entry: Entry,
    },
    WidgetDone {
        entry: Entry,
    },
    MessageHistory {
        history: Vec<Entry>,
    },

    // Resync
    RequestResyncReport {
        id: Snowflake,
    },
    CommitResync {
        timestamp: f64,
    },
    // Ready-state
    ReadinessUpdated {
        readiness: UserReadinessView,
    },
    RequestReadyConfirmation {
        request_id: Snowflake,
        video_id: Snowflake,
        deadline_ms: u64,
    },

    // Playback
    Play {
        request_id: Snowflake,
        track_audio: i32,
        track_subtitles: i32,
    },
    PlayAborted {
        request_id: Snowflake,
        non_confirmers: Vec<Snowflake>,
    },
    Pause,
    Resume,
    Seek {
        timestamp: f64,
    },

    // Playlist
    PlaylistUpdated {
        playlist: Vec<VideoEntry>,
    },
    VideoSelected {
        video_id: Snowflake,
    },

    // Errors
    Error {
        message: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping,
    RequestIdentity,
    RequestUsers,
    EditUserPermissions {
        target_user: Snowflake,
        permission: Permissions,
        granted: bool,
    },
    // Message
    RequestMessageHistory,
    SendMessage {
        content: String,
    },

    // Resync
    StartResync,
    SendResyncReport {
        state_id: Snowflake,
        timestamp: f64,
    },

    // Download
    DownloadProgress {
        widget_id: Snowflake,
        bytes_done: u64,
    },
    DownloadDone {
        widget_id: Snowflake,
    },

    // State tracking
    AssertReady {
        video_id: Snowflake,
        on_device: bool,
    },
    AssertReadyBulk {
        on_device: Vec<Snowflake>,
    },
    AssertPending {
        video_id: Snowflake,
    },
    Heartbeat,
    ConfirmReadyForPlay {
        request_id: Snowflake,
    },

    // Playback
    Play,
    RequestPause,
    RequestResume,
    RequestSeek {
        timestamp: f64,
    },

    // Playlist
    SelectVideo {
        video_id: Snowflake,
    },
    RequestPlaylist,
    SwapEntries {
        first: Snowflake,
        second: Snowflake,
    },
    SetDisplayName {
        video_id: Snowflake,
        display_name: String
    },
    SetAudioTrack {
        video_id: Snowflake,
        audio_track: i32
    },
    SetSubtitleTrack {
        video_id: Snowflake,
        subtitle_track: i32
    }
}
