use serde::Deserialize;

use crate::protocol::{Entry, Permissions, Snowflake, User, UserReadinessView, VideoEntry};

#[derive(Deserialize, Debug)]
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
