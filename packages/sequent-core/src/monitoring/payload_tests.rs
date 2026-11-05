// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use serde_json::json;

fn payload() -> ScopePayload {
    ScopePayload {
        totals: [(Measure::Voted, 3), (Measure::Registered, 5)].into(),
        posts: vec![PostRow {
            post_id: "e1".into(),
            post: "Madrid".into(),
            region: None,
            state: Some(PostState::Opened),
            counts: [(Measure::Posts, 1)].into(),
        }],
        series: vec![Bucket {
            start: "2026-05-04T00:00:00".into(),
            day: "2026-05-04".into(),
            utc_offset: "+08:00".into(),
            counts: [(Measure::Voted, 3)].into(),
        }],
        notices: vec![Notice::UnregisteredAttemptsExcluded],
        ..ScopePayload::default()
    }
}

#[test]
fn a_payload_serializes_the_same_way_every_time() {
    // Payloads are content-addressed: equal counts must hash equal.
    let text = serde_json::to_string(&payload()).unwrap();
    assert_eq!(
        text,
        r#"{"totals":{"registered":5,"voted":3},"posts":[{"post_id":"e1","post":"Madrid","state":"opened","counts":{"posts":1}}],"series":[{"start":"2026-05-04T00:00:00","day":"2026-05-04","utc_offset":"+08:00","counts":{"voted":3}}],"notices":["UNREGISTERED_ATTEMPTS_EXCLUDED"]}"#
    );
    let read: ScopePayload = serde_json::from_str(&text).unwrap();
    assert_eq!(read, payload());
}

/// During a rolling deploy a newer snapshot job can write what an older
/// Harvest does not know yet. The older reader drops what it cannot name
/// instead of refusing the whole payload.
#[test]
fn a_reader_skips_measures_states_and_notices_it_does_not_know() {
    let newer = json!({
        "totals": {"voted": 3, "registered": 5, "revoted": 9},
        "posts": [{
            "post_id": "e1", "post": "Madrid",
            "state": "suspended",
            "counts": {"posts": 1, "suspended": 1}
        }],
        "series": [{
            "start": "2026-05-04T00:00:00", "day": "2026-05-04",
            "utc_offset": "+08:00", "counts": {"voted": 3, "revoted": 1}
        }],
        "notices": ["UNREGISTERED_ATTEMPTS_EXCLUDED", "SOMETHING_NEW"]
    });
    let read: ScopePayload = serde_json::from_value(newer).unwrap();
    let mut expected = payload();
    expected.posts[0].state = None;
    assert_eq!(read, expected);
}
