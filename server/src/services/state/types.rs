use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Snowflake;

pub struct UserReadiness {
    pub videos: HashMap<Snowflake, VideoReadiness>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum VideoReadiness {
    OnDevice,
    Pending,
    NotStarted,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Ready,
    Partial,
    NotReady,
}

/// Wire shape. Verdict is derived at serialize time, never stored.
#[derive(Serialize, Debug)]
pub struct UserReadinessView {
    pub videos: HashMap<Snowflake, VideoReadiness>,
    pub verdict: Verdict,
}

pub enum HandshakeOutcome {
    Pending,
    AllConfirmed { video_id: Snowflake },
    AlreadyResolved,
}
