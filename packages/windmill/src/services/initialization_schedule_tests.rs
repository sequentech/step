// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::TimeZone;

#[test]
fn a_posts_close_is_its_own_else_the_event_wide_one() {
    let closes = vec![
        (None, "2028-05-08T11:00:00Z".to_string()),
        (
            Some("p1".to_string()),
            "2028-05-07T11:00:00+00:00".to_string(),
        ),
    ];
    assert_eq!(
        effective_close(&closes, "p1"),
        Some(Utc.with_ymd_and_hms(2028, 5, 7, 11, 0, 0).unwrap())
    );
    assert_eq!(
        effective_close(&closes, "p2"),
        Some(Utc.with_ymd_and_hms(2028, 5, 8, 11, 0, 0).unwrap())
    );
    assert_eq!(effective_close(&closes[1..], "p2"), None);
}

#[test]
fn waits_and_refusals_explain_themselves() {
    let waiting = WaitReason::Initialization(TransitionRefusal::CountriesNotInitialized {
        post: "Post A".to_string(),
        area_ids: vec!["b".to_string()],
        names: vec!["Country B".to_string()],
    });
    assert_eq!(waiting.code(), "countries-not-initialized");
    let text = wait_description("Post A", &waiting);
    assert!(
        text.starts_with("Scheduled opening of Post Post A waits for its initialization: "),
        "{text}"
    );
    assert!(text.contains("Not initialized yet: Country B."), "{text}");
    assert!(text.ends_with("The scheduler opens it at its first run after that."));

    let closed = WaitReason::AfterClose(Utc.with_ymd_and_hms(2028, 5, 8, 11, 0, 0).unwrap());
    assert_eq!(closed.code(), "after-close");
    assert_eq!(
        wait_description("Post A", &closed),
        "Scheduled opening of Post Post A not run: its voting period closed at 2028-05-08T11:00:00+00:00."
    );
}
