// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The period in which a voter can view a cast ballot is event configuration.
//! An event without it keeps unlimited checks, and a period that cannot be
//! read is an error instead of an open or a closed period.

#![cfg(feature = "default_features")]

use chrono::{DateTime, Utc};
use sequent_core::ballot::*;
use serde_json::json;

fn at(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

#[test]
fn policy_names_are_stable() {
    assert_eq!(
        serde_json::to_value(EChecksPeriodPolicy::UNLIMITED).unwrap(),
        json!("unlimited")
    );
    assert_eq!(
        serde_json::to_value(EChecksPeriodPolicy::UNTIL_DATE).unwrap(),
        json!("until-date")
    );
    assert_eq!(EChecksPeriodPolicy::UNTIL_DATE.to_string(), "until-date");
    assert_eq!(
        "unlimited".parse::<EChecksPeriodPolicy>().unwrap(),
        EChecksPeriodPolicy::UNLIMITED
    );
    assert_eq!(
        EChecksPeriodPolicy::default(),
        EChecksPeriodPolicy::UNLIMITED
    );
    assert!(
        serde_json::from_value::<EChecksPeriodPolicy>(json!("forever"))
            .is_err()
    );
}

#[test]
fn events_without_the_setting_keep_unlimited_checks() {
    let now = at("2028-06-08T00:00:00+08:00");
    for presentation in [
        None,
        Some(json!({})),
        Some(json!({"receipts": null})),
        Some(json!({"receipts": {}})),
        Some(json!({"receipts": {"checks_period_policy": "unlimited"}})),
        Some(json!({"receipts": {
            "checks_period_policy": "unlimited",
            "checks_available_until": "2020-01-01T00:00:00Z"
        }})),
    ] {
        assert_eq!(
            checks_period_from_presentation(presentation.as_ref(), now)
                .unwrap(),
            ChecksPeriod::Unlimited,
            "{presentation:?}"
        );
    }
}

#[test]
fn checks_stay_open_until_the_configured_instant_and_end_after_it() {
    let presentation = json!({"receipts": {
        "checks_period_policy": "until-date",
        "checks_available_until": "2028-06-07T23:59:00+08:00"
    }});
    let until = at("2028-06-07T15:59:00Z");

    assert_eq!(
        checks_period_from_presentation(
            Some(&presentation),
            at("2028-05-09T08:00:00+08:00")
        )
        .unwrap(),
        ChecksPeriod::OpenUntil(until)
    );
    assert_eq!(
        checks_period_from_presentation(Some(&presentation), until).unwrap(),
        ChecksPeriod::OpenUntil(until)
    );
    assert_eq!(
        checks_period_from_presentation(
            Some(&presentation),
            at("2028-06-07T15:59:01Z")
        )
        .unwrap(),
        ChecksPeriod::Ended(until)
    );
}

#[test]
fn a_period_that_cannot_be_read_is_an_error() {
    let now = at("2028-06-01T00:00:00Z");
    for receipts in [
        json!({"checks_period_policy": "until-date"}),
        json!({"checks_period_policy": "until-date", "checks_available_until": null}),
        json!({"checks_period_policy": "until-date", "checks_available_until": ""}),
        json!({"checks_period_policy": "until-date", "checks_available_until": "2028-06-07"}),
        json!({"checks_period_policy": "until-date", "checks_available_until": "2028-06-07T23:59"}),
        json!({"checks_period_policy": "forever"}),
        json!("until-date"),
    ] {
        let presentation = json!({ "receipts": receipts });
        assert!(
            checks_period_from_presentation(Some(&presentation), now).is_err(),
            "{presentation}"
        );
    }
}

#[test]
fn the_typed_presentation_reads_the_same_period() {
    let presentation: ElectionEventPresentation =
        serde_json::from_value(json!({"receipts": {
            "checks_period_policy": "until-date",
            "checks_available_until": "2028-06-07T23:59:00+08:00"
        }}))
        .unwrap();
    let receipts = presentation.receipts.clone().unwrap();
    assert_eq!(
        receipts.checks_period_policy,
        Some(EChecksPeriodPolicy::UNTIL_DATE)
    );
    assert_eq!(
        receipts
            .checks_period(at("2028-06-08T00:00:00+08:00"))
            .unwrap(),
        ChecksPeriod::Ended(at("2028-06-07T15:59:00Z"))
    );

    let legacy: ElectionEventPresentation =
        serde_json::from_value(json!({"show_cast_vote_logs": "show-logs-tab"}))
            .unwrap();
    assert_eq!(legacy.receipts, None);
}

#[test]
fn the_setting_does_not_change_the_serialized_ballot_style() {
    let with_receipts = ElectionEventPresentation {
        receipts: Some(ReceiptsPresentation {
            checks_period_policy: Some(EChecksPeriodPolicy::UNTIL_DATE),
            checks_available_until: Some(
                "2028-06-07T23:59:00+08:00".to_string(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(
        borsh::to_vec(&with_receipts).unwrap(),
        borsh::to_vec(&ElectionEventPresentation::default()).unwrap()
    );
}
