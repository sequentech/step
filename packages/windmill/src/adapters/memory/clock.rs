// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::clock::{Clock, IdGenerator};
use chrono::{DateTime, Duration, Local};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use uuid::Uuid;

/// A clock that only moves when a test advances it.
#[derive(Debug)]
pub struct FixedClock(Mutex<DateTime<Local>>);

impl FixedClock {
    pub fn at(now: DateTime<Local>) -> Self {
        Self(Mutex::new(now))
    }

    pub fn advance(&self, by: Duration) {
        let mut now = self.0.lock().expect("clock lock");
        *now += by;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Local> {
        *self.0.lock().expect("clock lock")
    }
}

/// Identifiers `00000000-0000-0000-0000-000000000001`, `…0002`, and so on.
#[derive(Debug, Default)]
pub struct SequentialIds(AtomicU64);

impl IdGenerator for SequentialIds {
    fn new_id(&self) -> Uuid {
        Uuid::from_u128(u128::from(self.0.fetch_add(1, Ordering::Relaxed)) + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_moves_only_when_advanced() {
        let start = Local::now();
        let clock = FixedClock::at(start);
        assert_eq!(clock.now(), start);
        clock.advance(Duration::seconds(90));
        assert_eq!(clock.now(), start + Duration::seconds(90));
    }

    #[test]
    fn sequential_ids_count_from_one() {
        let ids = SequentialIds::default();
        assert_eq!(ids.new_id(), Uuid::from_u128(1));
        assert_eq!(ids.new_id(), Uuid::from_u128(2));
    }
}
