// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A readiness verdict from authoritative snapshots. This module performs no
//! writes: the service records a signed statement only after every check passes.

use sequent_core::ballot::{ElectionStatus, PeriodDates, VotingStatus, VotingStatusChannel};
use sequent_core::types::voter_import::VotersImportedEvidence;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../tests/support/readiness_adversarial.rs"]
mod adversarial_tests;

pub const CHANNELS: [VotingStatusChannel; 4] = [
    VotingStatusChannel::ONLINE,
    VotingStatusChannel::KIOSK,
    VotingStatusChannel::EARLY_VOTING,
    VotingStatusChannel::TELEPHONE,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationEvidence {
    pub external_id: String,
    pub revision: u64,
    pub content_sha256: String,
    pub manifest_sha256: String,
    /// Canonical verified certificate identities, independent of approval order.
    pub approvers: BTreeSet<String>,
    /// Stable area names from the signed ballot designs.
    pub posts: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElectionEvidence {
    pub election_id: String,
    pub status: Option<ElectionStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationEvidence {
    pub publication_id: String,
    pub matches_manifest: bool,
    pub design_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEvidence {
    pub event_id: String,
    pub configuration: Option<ConfigurationEvidence>,
    pub voters: Option<VotersImportedEvidence>,
    pub voter_log_delivered: bool,
    pub current_voter_counts: BTreeMap<String, u64>,
    pub voter_import_in_progress: bool,
    pub status: Option<ElectionStatus>,
    pub elections: Vec<ElectionEvidence>,
    pub ballot_count: u64,
    pub received_ballot_count: u64,
    pub tally_session_count: u64,
    pub publication: Option<PublicationEvidence>,
    pub seal_at_close: bool,
    pub expected_boxes: BTreeSet<(String, String)>,
    pub sealed_boxes: BTreeSet<(String, String)>,
    pub all_channels_closed: bool,
    pub locked_down: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessEvidence {
    pub official: EventEvidence,
    pub tested: EventEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Configuration,
    CertifiedVoters,
    OfficialZero,
    Publication,
    TestedFinished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessCheck {
    pub kind: CheckKind,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    pub passed: bool,
    pub checked_at: String,
    pub checks: Vec<ReadinessCheck>,
    pub evidence: ReadinessEvidence,
    /// Set by the service only after the signed entry is on the official log.
    pub log_entry_id: Option<String>,
}

fn no_dates(dates: PeriodDates) -> bool {
    dates == PeriodDates::default()
}

pub fn never_started(status: &ElectionStatus) -> bool {
    CHANNELS.into_iter().all(|channel| {
        status.status_by_channel(channel) == VotingStatus::NOT_STARTED
            && no_dates(status.dates_by_channel(channel))
    })
}

fn same_configuration(official: &EventEvidence, tested: &EventEvidence) -> bool {
    match (&official.configuration, &tested.configuration) {
        (Some(left), Some(right)) => {
            official.event_id != tested.event_id
                && !left.external_id.is_empty()
                && !left.approvers.is_empty()
                && !left.posts.is_empty()
                && left.content_sha256.len() == 64
                && left.external_id == right.external_id
                && left.content_sha256 == right.content_sha256
                && left.approvers == right.approvers
                && left.posts == right.posts
        }
        _ => false,
    }
}

fn certified_voters(event: &EventEvidence) -> bool {
    match (&event.voters, &event.configuration) {
        (Some(voters), Some(configuration)) => {
            event.voter_log_delivered
                && !event.voter_import_in_progress
                && voters.is_certified()
                && voters.counts_by_post == event.current_voter_counts
                && voters
                    .counts_by_post
                    .keys()
                    .all(|post| configuration.posts.contains(post))
        }
        _ => false,
    }
}

pub fn evaluate(evidence: ReadinessEvidence, checked_at: String) -> ReadinessReport {
    let official = &evidence.official;
    let tested = &evidence.tested;
    let voters_match = certified_voters(official)
        && certified_voters(tested)
        && match (&official.voters, &tested.voters) {
            (Some(left), Some(right)) => {
                left.sha256.eq_ignore_ascii_case(&right.sha256)
                    && left.counts_by_post == right.counts_by_post
            }
            _ => false,
        };
    let official_zero = official.ballot_count == 0
        && official.received_ballot_count == 0
        && official.tally_session_count == 0
        && official.status.as_ref().is_some_and(never_started)
        && !official.elections.is_empty()
        && official
            .elections
            .iter()
            .all(|election| election.status.as_ref().is_some_and(never_started));
    let publication_matches = official
        .publication
        .as_ref()
        .is_some_and(|publication| publication.matches_manifest && publication.design_count > 0);
    let tested_finished = !tested.elections.is_empty()
        && tested.all_channels_closed
        && tested.seal_at_close
        && !tested.expected_boxes.is_empty()
        && tested.expected_boxes.is_subset(&tested.sealed_boxes);
    let facts = [
        (CheckKind::Configuration, same_configuration(official, tested),
            "Both events have the same signed content and verified approvers"),
        (CheckKind::CertifiedVoters, voters_match,
            "Both events have the same certified file, full current registry counts per Post and signed import evidence"),
        (CheckKind::OfficialZero, official_zero,
            "The official event has never opened voting and holds no ballots or tally sessions"),
        (CheckKind::Publication, publication_matches,
            "The official publication still matches its signed manifest"),
        (CheckKind::TestedFinished, tested_finished,
            "Every tested voting channel is closed and every expected ballot box is sealed"),
    ];
    let checks: Vec<ReadinessCheck> = facts
        .into_iter()
        .map(|(kind, passed, message)| ReadinessCheck {
            kind,
            passed,
            message: message.into(),
        })
        .collect();
    ReadinessReport {
        passed: checks.iter().all(|check| check.passed),
        checked_at,
        checks,
        evidence,
        log_entry_id: None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn event(id: &str, revision: u64) -> Value {
        json!({
            "event_id": id,
            "configuration": {
                "external_id": "approved-2028", "revision": revision,
                "content_sha256": "ab".repeat(32), "manifest_sha256": "cd".repeat(32),
                "approvers": ["cert-a", "cert-b"], "posts": ["Madrid", "Riyadh"]
            },
            "voters": {
                "import_id": format!("import-{id}"), "document_id": "document",
                "file_name": "certified.csv", "sha256": "ef".repeat(32),
                "declared_sha256": "ef".repeat(32),
                "counts_by_post": {"Madrid": 3, "Riyadh": 2},
                "imported_at": "2026-10-14T09:00:00Z"
            },
            "voter_log_delivered": true,
            "current_voter_counts": {"Madrid": 3, "Riyadh": 2},
            "voter_import_in_progress": false,
            "status": {},
            "elections": [{"election_id": format!("election-{id}"), "status": {}}],
            "ballot_count": 0, "received_ballot_count": 0, "tally_session_count": 0,
            "publication": {"publication_id": "publication", "matches_manifest": true, "design_count": 2},
            "seal_at_close": true,
            "expected_boxes": [[format!("election-{id}"), "area"]],
            "sealed_boxes": [[format!("election-{id}"), "area"]],
            "all_channels_closed": true, "locked_down": false
        })
    }

    pub(crate) fn evidence() -> Value {
        json!({"official": event("official", 8), "tested": event("tested", 9)})
    }

    fn check(value: Value) -> ReadinessReport {
        evaluate(
            serde_json::from_value(value).unwrap(),
            "2026-10-14T10:00:00Z".into(),
        )
    }

    #[test]
    fn different_revisions_and_approver_order_pass_for_identical_approved_content() {
        let mut value = evidence();
        value["tested"]["configuration"]["approvers"] = json!(["cert-b", "cert-a"]);
        value["tested"]["configuration"]["manifest_sha256"] = json!("12".repeat(32));
        let result = check(value);
        assert!(result.passed);
        assert_eq!(result.checks.len(), 5);
        assert!(result.log_entry_id.is_none());
        assert_eq!(result.evidence.tested.configuration.unwrap().revision, 9);
    }

    #[test]
    fn each_required_check_refuses_its_independent_mismatch() {
        let cases = [
            (
                "/tested/configuration/content_sha256",
                json!("00".repeat(32)),
                CheckKind::Configuration,
            ),
            (
                "/tested/configuration/approvers",
                json!(["other"]),
                CheckKind::Configuration,
            ),
            (
                "/tested/configuration/external_id",
                json!("other"),
                CheckKind::Configuration,
            ),
            (
                "/tested/voters/sha256",
                json!("00".repeat(32)),
                CheckKind::CertifiedVoters,
            ),
            (
                "/tested/voters/counts_by_post/Madrid",
                json!(4),
                CheckKind::CertifiedVoters,
            ),
            (
                "/tested/voter_log_delivered",
                json!(false),
                CheckKind::CertifiedVoters,
            ),
            (
                "/official/current_voter_counts/Madrid",
                json!(4),
                CheckKind::CertifiedVoters,
            ),
            (
                "/tested/voter_import_in_progress",
                json!(true),
                CheckKind::CertifiedVoters,
            ),
            ("/official/ballot_count", json!(1), CheckKind::OfficialZero),
            (
                "/official/received_ballot_count",
                json!(1),
                CheckKind::OfficialZero,
            ),
            (
                "/official/tally_session_count",
                json!(1),
                CheckKind::OfficialZero,
            ),
            (
                "/official/status/voting_status",
                json!("OPEN"),
                CheckKind::OfficialZero,
            ),
            (
                "/official/status/telephone_voting_status",
                json!("PAUSED"),
                CheckKind::OfficialZero,
            ),
            (
                "/official/publication/matches_manifest",
                json!(false),
                CheckKind::Publication,
            ),
            (
                "/tested/all_channels_closed",
                json!(false),
                CheckKind::TestedFinished,
            ),
            ("/tested/sealed_boxes", json!([]), CheckKind::TestedFinished),
            (
                "/tested/seal_at_close",
                json!(false),
                CheckKind::TestedFinished,
            ),
        ];
        for (path, replacement, expected) in cases {
            let mut value = evidence();
            // Paths ending in a not-yet-present status field are inserted explicitly.
            let (parent, key) = path.rsplit_once('/').unwrap();
            value.pointer_mut(parent).unwrap()[key] = replacement;
            let report = check(value);
            assert!(!report.passed, "{path}");
            assert!(
                report
                    .checks
                    .iter()
                    .any(|check| check.kind == expected && !check.passed),
                "{path}"
            );
        }
    }

    #[test]
    fn absent_evidence_and_empty_scopes_never_prove_readiness() {
        for path in [
            "/official/configuration",
            "/tested/voters",
            "/official/status",
            "/official/publication",
        ] {
            let mut value = evidence();
            *value.pointer_mut(path).unwrap() = Value::Null;
            assert!(!check(value).passed, "{path}");
        }
        for path in [
            "/official/elections",
            "/tested/elections",
            "/tested/expected_boxes",
        ] {
            let mut value = evidence();
            *value.pointer_mut(path).unwrap() = json!([]);
            assert!(!check(value).passed, "{path}");
        }
    }

    #[test]
    fn historical_voting_dates_and_uncertified_imports_fail() {
        let mut value = evidence();
        value["official"]["elections"][0]["status"] = json!({
            "voting_period_dates": {"first_started_at": "2026-10-13T10:00:00Z"}
        });
        assert!(!check(value).passed);
        let mut value = evidence();
        value["official"]["voters"]["declared_sha256"] = Value::Null;
        assert!(!check(value).passed);
    }
}
