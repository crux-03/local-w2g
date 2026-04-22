use serde::{Deserialize, Serialize};

use crate::services::message::Entry;

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    // Messages
    MessageCreated { entry: Entry },
    WidgetUpdated { entry: Entry},
    WidgetDone{ entry: Entry},

    // Errors
    Error { message: String },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    SendMessage { content: String },
}
