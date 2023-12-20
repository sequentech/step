pub mod braid;
pub mod electoral_log;

use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn timestamp() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}
