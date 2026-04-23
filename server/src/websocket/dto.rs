use serde::{Deserialize, Serialize};

use crate::{
    Snowflake,
    services::{message::Entry, state::UserReadinessView},
};

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
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

    // Resync
    RequestResyncReport {
        id: Snowflake,
    },
    CommitResync {
        timestamp: u32,
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
    },
    PlayAborted {
        request_id: Snowflake,
        non_confirmers: Vec<Snowflake>,
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
    // Message
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
    SelectVideo {
        video_id: Snowflake,
    },
}
