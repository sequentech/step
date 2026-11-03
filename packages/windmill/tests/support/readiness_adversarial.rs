// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::{json, Value};

fn valid() -> Value {
    let event = |id: &str| {
        json!({
            "event_id":id,
            "configuration":{"external_id":"approved", "revision":1,"content_sha256":"ab".repeat(32),"manifest_sha256":"cd".repeat(32),"approvers":["approver"],"areas":["Area A"]},
            "voters":{"import_id":format!("import-{id}"),"document_id":"document","file_name":"registry.csv","sha256":"ef".repeat(32),"declared_sha256":"ef".repeat(32),"counts_by_area":{"Area A":2},"imported_at":"2026-10-14T10:00:00Z"},
            "voter_log_delivered":true,"current_voter_counts":{"Area A":2},"voter_import_in_progress":false,"status":{},"elections":[{"election_id":format!("election-{id}"),"status":{}}],
            "ballot_count":0,"received_ballot_count":0,"tally_session_count":0,
            "publication":{"publication_id":"publication","matches_manifest":true,"design_count":1},
            "seal_at_close":true,"expected_boxes":[[format!("election-{id}"),"area"]],
            "sealed_boxes":[[format!("election-{id}"),"area"]],"all_channels_closed":true,"locked_down":false
        })
    };
    json!({"official":event("official"),"tested":event("tested")})
}

fn report(value: Value) -> ReadinessReport {
    evaluate(
        serde_json::from_value(value).unwrap(),
        "2026-10-14T11:00:00Z".into(),
    )
}

fn fails(value: Value, kind: CheckKind) {
    let result = report(value);
    assert!(!result.passed);
    assert!(result.log_entry_id.is_none());
    assert!(result
        .checks
        .iter()
        .any(|check| check.kind == kind && !check.passed));
}

#[test]
fn adversarial_every_channel_and_every_actual_history_field_blocks_official_zero() {
    assert!(report(valid()).passed);
    for prefix in ["", "kiosk_", "early_", "telephone_"] {
        for state in ["OPEN", "PAUSED", "CLOSED"] {
            let mut value = valid();
            value["official"]["status"][format!("{prefix}voting_status")] = json!(state);
            fails(value, CheckKind::OfficialZero);
        }
        for date in [
            "first_started_at",
            "last_started_at",
            "first_paused_at",
            "last_paused_at",
            "first_stopped_at",
            "last_stopped_at",
        ] {
            for event_level in [true, false] {
                let mut value = valid();
                let status = if event_level {
                    &mut value["official"]["status"]
                } else {
                    &mut value["official"]["elections"][0]["status"]
                };
                status[format!("{prefix}voting_period_dates")] =
                    json!({date:"2026-10-13T10:00:00Z"});
                fails(value, CheckKind::OfficialZero);
            }
        }
    }
}

#[test]
fn adversarial_matching_event_identity_and_partial_seals_cannot_pass() {
    let mut value = valid();
    value["tested"]["event_id"] = value["official"]["event_id"].clone();
    fails(value, CheckKind::Configuration);
    for sealed in [
        json!([]),
        json!([["election-tested", "area"]]),
        json!([
            ["another-election", "area"],
            ["election-tested", "another-area"]
        ]),
    ] {
        let mut value = valid();
        value["tested"]["expected_boxes"] = json!([
            ["election-tested", "area"],
            ["election-tested", "second-area"]
        ]);
        value["tested"]["sealed_boxes"] = sealed;
        fails(value, CheckKind::TestedFinished);
    }
}

#[test]
fn adversarial_undelivered_or_uncertified_or_wrong_area_evidence_fails_on_either_event() {
    for event in ["official", "tested"] {
        let mut value = valid();
        value[event]["voter_log_delivered"] = json!(false);
        fails(value, CheckKind::CertifiedVoters);
        let mut value = valid();
        value[event]["voters"]["declared_sha256"] = Value::Null;
        fails(value, CheckKind::CertifiedVoters);
    }
    // Matching counts and hashes between the two events still cannot prove
    // that voters belong to the signed configuration's actual area scope.
    let mut value = valid();
    for event in ["official", "tested"] {
        value[event]["voters"]["counts_by_area"] = json!({"Unknown area":2});
    }
    fails(value, CheckKind::CertifiedVoters);
}
