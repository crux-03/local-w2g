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
        timestamp: u32,
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
    Heartbeat,
    ConfirmReadyForPlay {
        request_id: Snowflake,
    },

    // Playback
    Play,

    // Playlist
    SelectVideo {
        video_id: Snowflake,
    },
    RequestPlaylist,
}
