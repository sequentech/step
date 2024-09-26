pub mod messages;

use std::time::{SystemTime, UNIX_EPOCH};

use crate::messages::newtypes::Timestamp;

pub fn get_schema_version() -> String {
    "1".to_string()
}

pub fn timestamp() -> Timestamp {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}
