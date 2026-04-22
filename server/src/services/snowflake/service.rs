use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const EPOCH: i64 = 1704067200000; // 2024-01-01 UTC in ms

const HOST_ID_BITS: i64 = 10;
const SEQUENCE_BITS: i64 = 12;

const MAX_HOST_ID: i64 = (1 << HOST_ID_BITS) - 1;
const MAX_SEQUENCE: i64 = (1 << SEQUENCE_BITS) - 1;

const HOST_ID_SHIFT: i64 = SEQUENCE_BITS;
const TIMESTAMP_SHIFT: i64 = SEQUENCE_BITS + HOST_ID_BITS;

pub struct SnowflakeService {
    host_id: i64,
    state: Mutex<SnowflakeState>,
}

struct SnowflakeState {
    last_timestamp: i64,
    sequence: i64,
}

impl SnowflakeService {
    pub fn new(host_id: i64) -> Self {
        assert!(
            host_id >= 0 && host_id <= MAX_HOST_ID,
            "host_id out of range"
        );

        Self {
            host_id,
            state: Mutex::new(SnowflakeState {
                last_timestamp: 0,
                sequence: 0,
            }),
        }
    }

    pub fn generate(&self) -> crate::Snowflake {
        let mut state = self.state.lock().unwrap();
        let mut timestamp = current_time_millis();

        if timestamp < state.last_timestamp {
            panic!("Clock moved backwards");
        }

        if timestamp == state.last_timestamp {
            state.sequence = (state.sequence + 1) & MAX_SEQUENCE;

            if state.sequence == 0 {
                while timestamp <= state.last_timestamp {
                    timestamp = current_time_millis();
                }
            }
        } else {
            state.sequence = 0;
        }

        state.last_timestamp = timestamp;

        let val = ((timestamp - EPOCH) << TIMESTAMP_SHIFT)
            | (self.host_id << HOST_ID_SHIFT)
            | state.sequence;
        crate::Snowflake(val)
    }
}

fn current_time_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
