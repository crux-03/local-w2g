use serde::Serialize;

use crate::Snowflake;

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: Snowflake,
    pub timestamp: i64,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EntryKind {
    Chat { sender: Snowflake, content: String },
    System { content: String },
    Widget { state: WidgetState, done: bool },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WidgetState {
    Upload {
        filename: String,
        bytes_done: u64,
        bytes_total: u64,
    },
    Download {
        reporter: Snowflake,
        filename: String,
        bytes_done: u64,
        bytes_total: u64,
    },
}
