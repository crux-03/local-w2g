use serde::{Deserialize, Serialize};

use crate::{Snowflake, services::message::Entry};

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    // Messages
    MessageCreated { entry: Entry },
    WidgetUpdated { entry: Entry },
    WidgetDone { entry: Entry },

    // Resync
    RequestResyncReport { id: Snowflake },
    CommitResync { timestamp: u32 },

    // Errors
    Error { message: String },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    // Message
    SendMessage { content: String },

    // Resync
    StartResync,
    SendResyncReport { state_id: Snowflake, timestamp: u32 },
}
