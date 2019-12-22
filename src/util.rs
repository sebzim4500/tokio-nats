use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn current_time_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}