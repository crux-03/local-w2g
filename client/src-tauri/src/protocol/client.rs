use serde::Serialize;

use crate::protocol::{Permissions, Snowflake};

#[derive(Serialize)]
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
        display_name: String,
    },
    SetAudioTrack {
        video_id: Snowflake,
        audio_track: i32,
    },
    SetSubtitleTrack {
        video_id: Snowflake,
        subtitle_track: i32,
    },
}
