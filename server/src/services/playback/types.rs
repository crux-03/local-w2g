use std::collections::HashMap;

use tokio::sync::Mutex;

use crate::Snowflake;

pub struct ResyncState {
    pub id: Snowflake,
    timestamps: Mutex<HashMap<Snowflake, Option<f64>>>,
}

impl ResyncState {
    pub fn new(id: Snowflake, users: Vec<Snowflake>) -> Self {
        let timestamps: HashMap<Snowflake, Option<f64>> =
            users.iter().map(|u| (*u, None)).collect();
        ResyncState {
            id,
            timestamps: Mutex::new(timestamps),
        }
    }

    /// Adds a timestamp
    ///
    /// Returns whether all timestamps have been filled
    pub async fn add_timestamp(&self, user: Snowflake, timestamp: f64) -> bool {
        let mut timestamps = self.timestamps.lock().await;
        timestamps.entry(user).insert_entry(Some(timestamp));
        timestamps.values().all(|t| t.is_some())
    }

    pub async fn get_min_timestamp(&self) -> Option<f64> {
        let timestamps = self.timestamps.lock().await;
        timestamps
            .values()
            .filter_map(|t| *t)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
    }
}

#[allow(dead_code)]
pub struct CollapsedResyncState {
    pub id: Snowflake,
    pub timestamp: f64,
}
