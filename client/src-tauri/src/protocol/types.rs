use bitflags::bitflags;
use std::{collections::HashMap, num::ParseIntError, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Snowflake ID — a 64-bit unique identifier encoding timestamp + worker + sequence.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Hash, Ord)]
pub struct Snowflake(pub i64);

impl std::fmt::Display for Snowflake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Snowflake {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Snowflake {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Accept both string and number from incoming JSON
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Snowflake;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a snowflake as string or integer")
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Snowflake, E> {
                Ok(Snowflake(v))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Snowflake, E> {
                Ok(Snowflake(v as i64))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Snowflake, E> {
                v.parse::<i64>().map(Snowflake).map_err(E::custom)
            }
        }
        d.deserialize_any(Visitor)
    }
}

impl FromStr for Snowflake {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Snowflake(s.parse::<i64>()?))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum VideoReadiness {
    OnDevice,
    Pending,
    NotStarted,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Ready,
    Partial,
    NotReady,
}

/// Wire shape. Verdict is derived at serialize time, never stored.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserReadinessView {
    pub user_id: Snowflake,
    pub videos: HashMap<Snowflake, VideoReadiness>,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: Snowflake,
    pub timestamp: i64,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EntryKind {
    Chat { sender: Snowflake, content: String },
    System { content: String },
    Widget { state: WidgetState, done: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WidgetState {
    Upload {
        uploader: Snowflake,
        filename: String,
        target: Snowflake,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Snowflake,
    pub display_name: Option<String>,
    pub permissions: Permissions,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Permissions: i64 {
        const MANAGE_PLAYBACK = 1 << 0;
        const SEND_MESSAGE = 1 << 1;
        const MANAGE_USERS = 1 << 2;
        const SEND_STATE = 1 << 3;
        const MANAGE_MEDIA = 1 << 4;
    }
}

impl Serialize for Permissions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = i64::deserialize(deserializer)?;
        Ok(Permissions::from_bits_truncate(bits))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEntry {
    pub id: Snowflake,
    pub display_name: String,
    pub audio_track: usize,
    pub subtitle_track: usize,
    /// Position in the ordered list, contiguous from 0. Carrier files written
    /// before this field existed deserialize to 0; the load-time sort by
    /// `(order, id)` then renumbers deterministically.
    #[serde(default)]
    pub order: usize,
}
