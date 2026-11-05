// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::ballot::{ElectionEventTimeZones, LogTimeZonePolicy};

fn event(
    configured: &[&str],
    primary: &str,
    logs: LogTimeZonePolicy,
) -> ElectionEventPresentation {
    ElectionEventPresentation {
        timezones: Some(ElectionEventTimeZones {
            configured: configured
                .iter()
                .map(|zone| zone.to_string())
                .collect(),
            primary: primary.to_owned(),
            logs,
        }),
        ..Default::default()
    }
}

fn election(zone: Option<&str>) -> ElectionPresentation {
    ElectionPresentation {
        timezone: zone.map(str::to_owned),
        ..Default::default()
    }
}

#[test]
fn an_election_uses_its_configured_zone() {
    let event = event(
        &["Asia/Manila", "Asia/Dubai"],
        "Asia/Manila",
        LogTimeZonePolicy::ELECTION,
    );
    assert_eq!(
        effective_time_zone(Some(&event), Some(&election(Some("Asia/Dubai")))),
        "Asia/Dubai"
    );
}

#[test]
fn an_election_without_a_zone_or_with_an_unconfigured_one_uses_the_primary() {
    let event = event(
        &["Europe/Madrid", "Atlantic/Canary"],
        "Europe/Madrid",
        LogTimeZonePolicy::ELECTION,
    );
    assert_eq!(
        effective_time_zone(Some(&event), Some(&election(None))),
        "Europe/Madrid"
    );
    assert_eq!(
        effective_time_zone(Some(&event), Some(&election(Some("Asia/Tokyo")))),
        "Europe/Madrid"
    );
    assert_eq!(effective_time_zone(Some(&event), None), "Europe/Madrid");
}

#[test]
fn an_event_without_timezones_uses_utc() {
    assert_eq!(effective_time_zone(None, None), DEFAULT_TIME_ZONE);
    let unset = ElectionEventPresentation::default();
    assert_eq!(
        effective_time_zone(Some(&unset), Some(&election(Some("Asia/Dubai")))),
        "UTC"
    );
}

#[test]
fn logs_follow_the_event_policy() {
    let primary = event(
        &["Asia/Manila", "Asia/Dubai"],
        "Asia/Manila",
        LogTimeZonePolicy::PRIMARY,
    );
    let per_election = event(
        &["Asia/Manila", "Asia/Dubai"],
        "Asia/Manila",
        LogTimeZonePolicy::ELECTION,
    );
    let dubai = election(Some("Asia/Dubai"));
    assert_eq!(log_time_zone(Some(&primary), Some(&dubai)), "Asia/Manila");
    assert_eq!(
        log_time_zone(Some(&per_election), Some(&dubai)),
        "Asia/Dubai"
    );
}

#[test]
fn legacy_presentations_use_restrictive_lifecycle_and_election_log_defaults() {
    let legacy: ElectionEventPresentation = serde_json::from_str("{}").unwrap();
    let defaults = lifecycle_policies(Some(&legacy));
    assert_eq!(defaults.initialization_scope, InitializationScope::POST);
    assert_eq!(
        defaults.unsigned_scheduled_close,
        UnsignedScheduledClosePolicy::REFUSE
    );
    assert_eq!(lifecycle_policies(None), defaults);
    assert_eq!(restrictive_lifecycle_policies(), defaults);

    let zones: ElectionEventTimeZones = serde_json::from_str(
        r#"{"configured":["Asia/Manila"],"primary":"Asia/Manila"}"#,
    )
    .unwrap();
    assert_eq!(zones.logs, LogTimeZonePolicy::ELECTION);
    assert_eq!(log_time_zone(None, None), "UTC");
}

#[test]
fn configured_lifecycle_policies_are_preserved_without_replacing_missing_fields(
) {
    let event: ElectionEventPresentation = serde_json::from_str(
        r#"{"lifecycle_policies":{"initialization_scope":"event","unsigned_scheduled_close":"run-as-system"}}"#,
    )
    .unwrap();
    let policy = lifecycle_policies(Some(&event));
    assert_eq!(policy.initialization_scope, InitializationScope::EVENT);
    assert_eq!(
        policy.unsigned_scheduled_close,
        UnsignedScheduledClosePolicy::RUN_AS_SYSTEM
    );
    assert_eq!(
        serde_json::to_value(&policy).unwrap(),
        serde_json::json!({
            "initialization_scope": "event",
            "unsigned_scheduled_close": "run-as-system"
        })
    );

    let partial: LifecyclePolicies =
        serde_json::from_str(r#"{"initialization_scope":"post-and-country"}"#)
            .unwrap();
    assert_eq!(
        partial.initialization_scope,
        InitializationScope::POST_AND_COUNTRY
    );
    assert_eq!(
        partial.unsigned_scheduled_close,
        UnsignedScheduledClosePolicy::REFUSE
    );
    for invalid in [
        r#"{"initialization_scope":"office"}"#,
        r#"{"unsigned_scheduled_close":true}"#,
        r#"{"unsigned_scheduled_close":null}"#,
    ] {
        assert!(serde_json::from_str::<LifecyclePolicies>(invalid).is_err());
    }
}

#[test]
fn policy_wire_values_and_signed_binary_discriminants_remain_stable() {
    use borsh::BorshDeserialize;
    use std::str::FromStr;

    for (policy, wire, binary) in [
        (InitializationScope::POST, "post", 0u8),
        (InitializationScope::EVENT, "event", 1),
        (InitializationScope::POST_AND_COUNTRY, "post-and-country", 2),
    ] {
        assert_eq!(
            serde_json::to_value(&policy).unwrap(),
            serde_json::json!(wire)
        );
        assert_eq!(
            serde_json::from_value::<InitializationScope>(serde_json::json!(
                wire
            ))
            .unwrap(),
            policy
        );
        assert_eq!(InitializationScope::from_str(wire).unwrap(), policy);
        assert_eq!(policy.to_string(), wire);
        assert_eq!(borsh::to_vec(&policy).unwrap(), vec![binary]);
        assert_eq!(
            InitializationScope::try_from_slice(&[binary]).unwrap(),
            policy
        );
    }
    for (policy, wire, binary) in [
        (UnsignedScheduledClosePolicy::REFUSE, "refuse", 0u8),
        (
            UnsignedScheduledClosePolicy::RUN_AS_SYSTEM,
            "run-as-system",
            1,
        ),
    ] {
        assert_eq!(
            serde_json::to_value(&policy).unwrap(),
            serde_json::json!(wire)
        );
        assert_eq!(
            serde_json::from_value::<UnsignedScheduledClosePolicy>(
                serde_json::json!(wire)
            )
            .unwrap(),
            policy
        );
        assert_eq!(
            UnsignedScheduledClosePolicy::from_str(wire).unwrap(),
            policy
        );
        assert_eq!(policy.to_string(), wire);
        assert_eq!(borsh::to_vec(&policy).unwrap(), vec![binary]);
        assert_eq!(
            UnsignedScheduledClosePolicy::try_from_slice(&[binary]).unwrap(),
            policy
        );
    }
    for (policy, wire, binary) in [
        (LogTimeZonePolicy::PRIMARY, "primary", 0u8),
        (LogTimeZonePolicy::ELECTION, "election", 1),
    ] {
        assert_eq!(
            serde_json::to_value(&policy).unwrap(),
            serde_json::json!(wire)
        );
        assert_eq!(
            serde_json::from_value::<LogTimeZonePolicy>(serde_json::json!(
                wire
            ))
            .unwrap(),
            policy
        );
        assert_eq!(LogTimeZonePolicy::from_str(wire).unwrap(), policy);
        assert_eq!(policy.to_string(), wire);
        assert_eq!(borsh::to_vec(&policy).unwrap(), vec![binary]);
        assert_eq!(
            LogTimeZonePolicy::try_from_slice(&[binary]).unwrap(),
            policy
        );
    }

    let policy = LifecyclePolicies {
        initialization_scope: InitializationScope::POST_AND_COUNTRY,
        unsigned_scheduled_close: UnsignedScheduledClosePolicy::RUN_AS_SYSTEM,
    };
    assert_eq!(borsh::to_vec(&policy).unwrap(), vec![2, 1]);
    assert_eq!(LifecyclePolicies::try_from_slice(&[2, 1]).unwrap(), policy);
    assert!(LifecyclePolicies::try_from_slice(&[2, 2]).is_err());
    assert!(InitializationScope::try_from_slice(&[3]).is_err());
    assert!(LogTimeZonePolicy::try_from_slice(&[2]).is_err());

    let zones = ElectionEventTimeZones {
        configured: vec!["UTC".to_owned()],
        primary: "UTC".to_owned(),
        logs: LogTimeZonePolicy::ELECTION,
    };
    // Vec count, UTF-8 lengths and enum discriminant form the signed payload.
    let expected = vec![
        1, 0, 0, 0, 3, 0, 0, 0, b'U', b'T', b'C', 3, 0, 0, 0, b'U', b'T', b'C',
        1,
    ];
    assert_eq!(borsh::to_vec(&zones).unwrap(), expected);
    assert_eq!(
        ElectionEventTimeZones::try_from_slice(&expected).unwrap(),
        zones
    );
    assert!(ElectionEventTimeZones::try_from_slice(
        &expected[..expected.len() - 1]
    )
    .is_err());
}

#[test]
fn whitespace_and_blank_zone_inputs_follow_the_documented_fallback() {
    let configured = event(
        &["Asia/Manila"],
        " Asia/Manila ",
        LogTimeZonePolicy::ELECTION,
    );
    assert_eq!(primary_time_zone(Some(&configured)), "Asia/Manila");
    assert_eq!(
        effective_time_zone(
            Some(&configured),
            Some(&election(Some(" Asia/Manila ")))
        ),
        "Asia/Manila"
    );
    let blank = event(&[], "  ", LogTimeZonePolicy::PRIMARY);
    assert_eq!(primary_time_zone(Some(&blank)), "UTC");
    assert_eq!(log_time_zone(Some(&blank), None), "UTC");
}

#[test]
fn scheduled_outcome_reasons_preserve_the_public_api_contract() {
    use crate::types::scheduled_outcome::{
        CheckId, Explanation, ScheduledOutcomeKind,
    };
    use serde_json::json;
    use std::str::FromStr;

    let wire = json!({
        "outcome": "waiting-for-initialization",
        "checks": [{
            "id": "initialization",
            "current": {"message_key": "lifecycle.initialization.required"},
            "published": null,
            "allows": false
        }],
        "deciding": "initialization",
        "next_step": {"message_key": "lifecycle.initialize", "params": {"post": "Madrid"}}
    });
    let explanation: Explanation =
        serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        explanation.outcome,
        ScheduledOutcomeKind::WaitingForInitialization
    );
    assert_eq!(explanation.deciding, CheckId::Initialization);
    assert!(explanation.authorized_by.is_none());
    assert!(explanation.checks[0].current.params.is_empty());
    let mut expected = wire;
    expected["checks"][0]["current"]["params"] = json!({});
    assert_eq!(serde_json::to_value(&explanation).unwrap(), expected);
    for wire in [
        "runs",
        "runs-unsigned",
        "refused",
        "waiting-for-initialization",
    ] {
        let outcome: ScheduledOutcomeKind =
            serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(serde_json::to_value(&outcome).unwrap(), json!(wire));
        assert_eq!(ScheduledOutcomeKind::from_str(wire).unwrap(), outcome);
        assert_eq!(outcome.to_string(), wire);
    }
    assert!(
        serde_json::from_value::<ScheduledOutcomeKind>(json!("unknown"))
            .is_err()
    );
}

#[test]
fn legacy_lifecycle_snapshots_load_without_new_initialization_evidence() {
    use crate::types::scheduled_outcome::{AuthorizedBy, LifecycleSnapshot};
    use serde_json::json;

    let wire = json!({
        "policies": {},
        "open_voting": {"required": true, "signatures": 2, "digest": "open"},
        "close_voting": {"required": false, "signatures": null, "digest": "close"},
        "schedule": [{
            "scheduled_event_id": "opening-1",
            "event_processor": "START_VOTING_PERIOD",
            "election_id": "post-1",
            "scheduled_date": "2028-04-08T20:00:00Z",
            "local": "2028-04-09T04:00:00",
            "timezone": "Asia/Manila",
            "voting_channels": ["ONLINE"],
            "fingerprint": "scheduled-fingerprint"
        }]
    });
    let snapshot: LifecycleSnapshot =
        serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(snapshot.policies, restrictive_lifecycle_policies());
    assert!(snapshot.initialization_report_policies.is_empty());
    assert!(snapshot.initialization_countries.is_none());
    let mut expected = wire;
    expected["policies"] = json!({
        "initialization_scope": "post",
        "unsigned_scheduled_close": "refuse"
    });
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), expected);

    let authorization = json!({
        "request_id": "approval-1",
        "code": "TX6R2YA5",
        "signers": ["Trustee One", "Trustee Two"]
    });
    let authorized: AuthorizedBy =
        serde_json::from_value(authorization.clone()).unwrap();
    assert_eq!(authorized.signers.len(), 2);
    assert_eq!(serde_json::to_value(&authorized).unwrap(), authorization);
}
