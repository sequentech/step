// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`]: the messages the snapshot job sends expire after
//! one snapshot interval.

use super::*;
use celery::protocol::Message;
use sequent_core::monitoring::cadence::{
    Cadence, MAX_SNAPSHOT_INTERVAL_SECONDS, MIN_SNAPSHOT_INTERVAL_SECONDS,
};

/// Seconds from now until the message expires.
fn expires_in(message: &Message) -> i64 {
    let expires = message.headers.expires.expect("the message expires");
    (expires - Utc::now()).num_seconds()
}

#[test]
fn an_event_pass_expires_after_one_interval() {
    for interval in [5, 30, 600] {
        let message = Message::try_from(event_pass(Uuid::new_v4(), Uuid::new_v4(), interval))
            .expect("a message");
        let left = expires_in(&message);
        assert!(
            (interval as i64 - 2..=interval as i64).contains(&left),
            "{interval}: expires in {left} s"
        );
    }
}

#[test]
fn beats_fan_out_carries_and_expires_after_its_interval() {
    let cadence = Cadence::parse(Some("120"), None);
    let message = Message::try_from(scheduled_fan_out(&cadence)).expect("a message");
    let body: serde_json::Value = serde_json::from_slice(&message.raw_body).expect("a JSON body");
    assert_eq!(body[1]["interval_seconds"], 120, "{body}");
    let left = expires_in(&message);
    assert!((118..=120).contains(&left), "expires in {left} s");
}

#[test]
fn the_fan_out_keeps_the_interval_within_its_bounds() {
    assert_eq!(fan_out_interval(Some(60)), 60);
    assert_eq!(fan_out_interval(Some(0)), MIN_SNAPSHOT_INTERVAL_SECONDS);
    assert_eq!(
        fan_out_interval(Some(u64::MAX)),
        MAX_SNAPSHOT_INTERVAL_SECONDS
    );
    // A message from a beat that sent no interval: this worker's own.
    assert_eq!(
        fan_out_interval(None),
        cadence::configured().snapshot_interval.seconds
    );
}

#[test]
fn pruning_an_event_takes_the_lock_its_passes_take() {
    let event = EventRef {
        tenant_id: Uuid::new_v4(),
        election_event_id: Uuid::new_v4(),
    };
    assert_eq!(
        snapshot_lock_key(event),
        format!(
            "monitoring_snapshot-{}-{}",
            event.tenant_id, event.election_event_id
        )
    );
}
