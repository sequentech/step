// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::clock::{Clock, IdGenerator};
use chrono::{DateTime, Local};
use sequent_core::services::date::ISO8601;
use uuid::Uuid;

/// The system clock, as used by `ISO8601::now`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Local> {
        ISO8601::now()
    }
}

/// Random version 4 UUIDs.
#[derive(Clone, Copy, Debug, Default)]
pub struct RandomIds;

impl IdGenerator for RandomIds {
    fn new_id(&self) -> Uuid {
        Uuid::new_v4()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_reads_the_current_time() {
        let before = Local::now();
        let now = SystemClock.now();
        assert!(before <= now && now <= Local::now());
    }

    #[test]
    fn random_ids_are_distinct_version_4_uuids() {
        let (first, second) = (RandomIds.new_id(), RandomIds.new_id());
        assert_ne!(first, second);
        assert_eq!(first.get_version_num(), 4);
    }
}
