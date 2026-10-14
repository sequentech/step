// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The configuration cache: an event's documents are read and checked once
//! per configuration generation, however many viewers ask.

use super::ConfigCache;
use std::sync::Arc;
use uuid::Uuid;
use windmill::services::monitoring::config_store::EventRef;

fn event(n: u128) -> EventRef {
    EventRef {
        tenant_id: Uuid::from_u128(1),
        election_event_id: Uuid::from_u128(n),
    }
}

#[test]
fn a_generation_is_kept_and_another_one_is_not_it() {
    let cache: ConfigCache<String> = ConfigCache::new(8);
    assert!(cache.get(event(1), 3).is_none());
    let kept = cache.put(event(1), 3, "generation 3".to_string());
    assert_eq!(*kept, "generation 3");
    let again = cache.get(event(1), 3).expect("kept");
    assert!(Arc::ptr_eq(&kept, &again));
    // A save raised the generation: what was kept is not the live one.
    assert!(cache.get(event(1), 4).is_none());
    // Nor is it another event's.
    assert!(cache.get(event(2), 3).is_none());
    assert_eq!(cache.loads(), 1);
}

#[test]
fn the_least_recently_used_generation_goes_first() {
    let cache: ConfigCache<i64> = ConfigCache::new(2);
    cache.put(event(1), 1, 1);
    cache.put(event(2), 1, 2);
    assert!(cache.get(event(1), 1).is_some());
    cache.put(event(3), 1, 3);
    assert!(cache.get(event(1), 1).is_some());
    assert!(cache.get(event(2), 1).is_none());
    assert!(cache.get(event(3), 1).is_some());
    assert_eq!(cache.loads(), 3);
}

#[test]
fn a_zero_capacity_still_keeps_one() {
    let cache: ConfigCache<i64> = ConfigCache::new(0);
    cache.put(event(1), 1, 1);
    assert!(cache.get(event(1), 1).is_some());
}
