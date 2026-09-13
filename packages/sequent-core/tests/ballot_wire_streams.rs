// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Borsh is part of the signed ballot format. Small golden records pin byte
//! order; larger records exercise truncation and sink failures at every byte.

use sequent_core::ballot::*;
use sequent_core::types::ceremonies::TallySessionResolutionData;
use serde_json::json;

#[path = "support/borsh.rs"]
mod wire;

macro_rules! record_contract {
    ($test:ident, $record:ty, $value:expr, $bytes:expr) => {
        #[test]
        fn $test() {
            let value: $record = serde_json::from_value($value).unwrap();
            wire::assert_stream_contract(&value, $bytes);
        }
    };
}

// String lengths are u32 LE, Option tags are 0/1 and field order is positional.
// These expected bytes are deliberately literal, not produced by another call
// to Borsh. A matching serializer/deserializer bug must still fail these tests.
record_contract!(
    public_key,
    PublicKeyConfig,
    json!({"public_key": "pk", "is_demo": true}),
    &[2, 0, 0, 0, b'p', b'k', 1]
);
record_contract!(
    candidate_url,
    CandidateUrl,
    json!({"url": "u", "kind": "k", "title": null, "is_image": false}),
    &[1, 0, 0, 0, b'u', 1, 1, 0, 0, 0, b'k', 0, 0]
);
record_contract!(
    materials,
    ElectionEventMaterials,
    json!({"policy": "mandatory_for_voting"}),
    &[1, 2]
);
record_contract!(
    results_policy,
    ResultsWebsitePolicy,
    json!({"status": "enabled", "access": "authenticated", "visibility_scope": "area_based"}),
    &[0, 1, 1]
);
record_contract!(
    area_presentation,
    AreaPresentation,
    json!({"allow_early_voting": "allow_early_voting"}),
    &[1, 0]
);
record_contract!(
    voting_dates,
    VotingPeriodDates,
    json!({"start_date": "s", "end_date": "e"}),
    &[1, 1, 0, 0, 0, b's', 1, 1, 0, 0, 0, b'e']
);
record_contract!(
    scheduled_dates,
    ScheduledEventDates,
    json!({"scheduled_at": "s", "stopped_at": null}),
    &[1, 1, 0, 0, 0, b's', 0]
);
record_contract!(
    custom_urls,
    CustomUrls,
    json!({"login": "l", "enrollment": null, "saml": "s"}),
    &[1, 1, 0, 0, 0, b'l', 0, 1, 1, 0, 0, 0, b's']
);
record_contract!(weight, Weight, json!(258), &[1, 2, 1, 0, 0, 0, 0, 0, 0]);
record_contract!(
    event_statistics,
    ElectionEventStatistics,
    json!({"num_emails_sent": 258, "num_sms_sent": null}),
    &[1, 2, 1, 0, 0, 0, 0, 0, 0, 0]
);
record_contract!(
    election_statistics,
    ElectionStatistics,
    json!({"num_emails_sent": null, "num_sms_sent": 258}),
    &[0, 1, 2, 1, 0, 0, 0, 0, 0, 0]
);

#[test]
fn nested_ballot_style_propagates_failures_inside_contests_and_presentations() {
    let style: BallotStyle = serde_json::from_value(json!({
        "id": "style", "tenant_id": "tenant", "election_event_id": "event",
        "election_id": "election", "area_id": "area", "num_allowed_revotes": 2,
        "public_key": {"public_key": "synthetic-key", "is_demo": true},
        "area_presentation": {"allow_early_voting": "allow_early_voting"},
        "area_annotations": {"weight": 258, "tally_operation": "aggregate-results"},
        "multi_contest_encoding_mode": "expanded-capacity",
        "election_dates": {"first_started_at": "2026-09-13T12:00:00Z",
            "scheduled_event_dates": {"close": {"scheduled_at": "2026-09-13T18:00:00Z"}}},
        "election_event_presentation": {
            "materials": {"policy": "mandatory_for_voting"},
            "language_conf": {"enabled_language_codes": ["en", "fr"], "default_language_code": "fr", "language_detection_policy": "force-default"},
            "voting_portal_countdown_policy": {"policy": "COUNTDOWN_WITH_ALERT", "countdown_anticipation_secs": 300, "countdown_alert_anticipation_secs": 30},
            "custom_urls": {"login": "https://login.invalid"},
            "ceremonies_policy": "automated-ceremonies",
            "voting_portal_datetime_format": {"custom": "yyyy-MM-dd"}
        },
        "election_presentation": {
            "dates": {"start_date": "2026-09-13", "end_date": "2026-09-14"},
            "audit_button_cfg": "show-in-help", "initialization_report_policy": "required",
            "blank_ballots_policy": "enabled"
        },
        "contests": [{
            "id": "contest", "tenant_id": "tenant", "election_event_id": "event", "election_id": "election",
            "min_votes": 1, "max_votes": 2, "winning_candidates_num": 1, "is_encrypted": true,
            "counting_algorithm": "plurality-at-large",
            "candidates": [{"id": "candidate", "tenant_id": "tenant", "election_event_id": "event",
                "election_id": "election", "contest_id": "contest", "name": "Élodie",
                "presentation": {"is_write_in": true, "sort_order": -1,
                    "urls": [{"url": "https://candidate.invalid", "is_image": false}]}}],
            "presentation": {"allow_writeins": true, "candidates_order": "custom",
                "types_presentation": {"list": {"name": "List", "sort_order": 1,
                    "subtypes_presentation": {"local": {"name": "Local", "sort_order": 2}}}}}
        }]
    })).unwrap();
    // These are stream-failure properties, not a second golden-byte test. The
    // valid object controls the decode outcome; the sink supplies an independent
    // error, even when failure occurs deep inside nested generated code.
    wire::assert_stream_contract(&style, &borsh::to_vec(&style).unwrap());

    let election: Election = serde_json::from_value(json!({
        "id": "election", "election_event_id": "event", "tenant_id": "tenant",
        "name": "Council", "contests": style.contests,
        "presentation": style.election_presentation
    }))
    .unwrap();
    wire::assert_stream_contract(&election, &borsh::to_vec(&election).unwrap());
}

#[test]
fn tally_resolution_stream_keeps_round_votes_candidates_and_selected_identity()
{
    let resolution: TallySessionResolutionData = serde_json::from_value(json!({
        "round_number": 2, "tied_candidate_ids": ["a", "b"], "vote_count": 258,
        "method_used": "ExternalProcedure", "resolved_by_candidate_id": "b"
    })).unwrap();
    wire::assert_stream_contract(
        &resolution,
        &[
            1, 2, 0, 0, 0, 0, 0, 0, 0, // Some(round 2)
            2, 0, 0, 0, 1, 0, 0, 0, b'a', 1, 0, 0, 0, b'b', // two IDs
            2, 1, 0, 0, 0, 0, 0, 0, // 258 votes
            1, 1, 1, 0, 0, 0, b'b', // external procedure, Some("b")
        ],
    );
}
