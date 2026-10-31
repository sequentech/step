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

#[cfg(test)]
#[path = "../../tests/support/readiness_review.rs"]
mod review_tests;

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
    #[serde(alias = "posts")]
    pub areas: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElectionEvidence {
    pub election_id: String,
    pub status: Option<ElectionStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationEvidence {
    /// Newest applicable publication, retained for older report consumers.
    pub publication_id: String,
    /// All active publications contributing designs, including per-election ones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub publication_ids: Vec<String>,
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

/// Which compared event a finding was observed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventRole {
    Official,
    Tested,
    /// A difference between the two events.
    Both,
}

/// The failed predicate, a stable language-independent contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCode {
    SameEvent,
    ConfigurationMissing,
    ConfigurationIncomplete,
    ExternalIdMismatch,
    ContentMismatch,
    ApproversMismatch,
    AreasMismatch,
    VoterImportMissing,
    VoterImportInProgress,
    VoterFileNotCertified,
    VoterReceiptNotSigned,
    RegistryCountChanged,
    VoterAreaNotConfigured,
    VoterFileMismatch,
    VoterCountMismatch,
    BallotsPresent,
    ReceivedBallotsPresent,
    TallySessionsPresent,
    StatusMissing,
    VotingStarted,
    NoElections,
    PublicationMissing,
    PublicationMismatch,
    PublicationEmpty,
    ChannelsOpen,
    SealAtCloseOff,
    NoExpectedBoxes,
    BoxesUnsealed,
}

/// Why a check failed, derived only from the evaluated evidence. It carries
/// area names, election IDs, counts and digests, never voter data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessFinding {
    pub code: FindingCode,
    pub event: EventRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub election_id: Option<String>,
    /// Observed values, for translated views.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, String>,
    pub message: String,
}

impl ReadinessFinding {
    fn new(code: FindingCode, event: EventRole, message: impl Into<String>) -> Self {
        ReadinessFinding {
            code,
            event,
            area: None,
            election_id: None,
            values: BTreeMap::new(),
            message: message.into(),
        }
    }

    fn area(mut self, area: &str) -> Self {
        self.area = Some(area.into());
        self
    }

    fn election(mut self, election_id: &str) -> Self {
        self.election_id = Some(election_id.into());
        self
    }

    fn value(mut self, name: &str, value: impl ToString) -> Self {
        self.values.insert(name.into(), value.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessCheck {
    pub kind: CheckKind,
    pub passed: bool,
    /// The condition checked. Stable, so older reports read the same.
    pub message: String,
    /// Why a failed check failed. Empty on a pass and in older reports.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<ReadinessFinding>,
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

fn role_name(role: EventRole) -> &'static str {
    match role {
        EventRole::Official => "official",
        EventRole::Tested => "tested",
        EventRole::Both => "both",
    }
}

fn area_label(area: &str) -> String {
    if area.trim().is_empty() {
        "no or an unknown area".into()
    } else {
        format!("area {area}")
    }
}

fn configuration_findings(
    official: &EventEvidence,
    tested: &EventEvidence,
) -> Vec<ReadinessFinding> {
    use FindingCode::*;
    let mut findings = Vec::new();
    if official.event_id == tested.event_id {
        findings.push(ReadinessFinding::new(
            SameEvent,
            EventRole::Both,
            "The official and tested events are the same event",
        ));
    }
    let (left, right) = match (&official.configuration, &tested.configuration) {
        (Some(left), Some(right)) => (left, right),
        (left, right) => {
            for (role, configuration) in [(EventRole::Official, left), (EventRole::Tested, right)] {
                if configuration.is_none() {
                    findings.push(ReadinessFinding::new(
                        ConfigurationMissing,
                        role,
                        format!(
                            "The {} event was not imported from a signed configuration package",
                            role_name(role)
                        ),
                    ));
                }
            }
            return findings;
        }
    };
    for (field, missing) in [
        ("external_id", left.external_id.is_empty()),
        ("approvers", left.approvers.is_empty()),
        ("areas", left.areas.is_empty()),
        ("content_sha256", left.content_sha256.len() != 64),
    ] {
        if missing {
            findings.push(
                ReadinessFinding::new(
                    ConfigurationIncomplete,
                    EventRole::Official,
                    format!("The official event's signed configuration has no valid {field}"),
                )
                .value("field", field),
            );
        }
    }
    if left.external_id != right.external_id {
        findings.push(
            ReadinessFinding::new(
                ExternalIdMismatch,
                EventRole::Both,
                format!(
                    "The configurations differ: the official event is {} and the tested event is {}",
                    left.external_id, right.external_id
                ),
            )
            .value("official", &left.external_id)
            .value("tested", &right.external_id),
        );
    }
    if left.content_sha256 != right.content_sha256 {
        findings.push(
            ReadinessFinding::new(
                ContentMismatch,
                EventRole::Both,
                format!(
                    "The signed content differs: SHA-256 {} on the official event, {} on the tested event",
                    left.content_sha256, right.content_sha256
                ),
            )
            .value("official", &left.content_sha256)
            .value("tested", &right.content_sha256),
        );
    }
    if left.approvers != right.approvers {
        let only_official = left.approvers.difference(&right.approvers).count();
        let only_tested = right.approvers.difference(&left.approvers).count();
        findings.push(
            ReadinessFinding::new(
                ApproversMismatch,
                EventRole::Both,
                format!(
                    "The verified approvers differ: {only_official} only on the official event, {only_tested} only on the tested event"
                ),
            )
            .value("only_official", only_official)
            .value("only_tested", only_tested),
        );
    }
    for (role, area) in left
        .areas
        .difference(&right.areas)
        .map(|area| (EventRole::Official, area))
        .chain(
            right
                .areas
                .difference(&left.areas)
                .map(|area| (EventRole::Tested, area)),
        )
    {
        findings.push(
            ReadinessFinding::new(
                AreasMismatch,
                role,
                format!(
                    "Area {area} is in the {} event's signed content only",
                    role_name(role)
                ),
            )
            .area(area),
        );
    }
    findings
}

fn certification_findings(event: &EventEvidence, role: EventRole) -> Vec<ReadinessFinding> {
    use FindingCode::*;
    let name = role_name(role);
    let mut findings = Vec::new();
    if event.voter_import_in_progress {
        findings.push(ReadinessFinding::new(
            VoterImportInProgress,
            role,
            format!("A voter import is still running on the {name} event"),
        ));
    }
    let Some(voters) = &event.voters else {
        findings.push(ReadinessFinding::new(
            VoterImportMissing,
            role,
            format!("The {name} event has no certified voter import"),
        ));
        return findings;
    };
    if !event.voter_log_delivered {
        findings.push(
            ReadinessFinding::new(
                VoterReceiptNotSigned,
                role,
                format!(
                    "The {name} event's log has no matching signed VotersImported entry for import {}",
                    voters.import_id
                ),
            )
            .value("import_id", &voters.import_id),
        );
    }
    if !voters.is_certified() {
        let (reason, explanation) = match &voters.declared_sha256 {
            None => ("declared_sha256_missing", "no declared SHA-256"),
            Some(declared) if !declared.eq_ignore_ascii_case(&voters.sha256) => (
                "declared_sha256_mismatch",
                "a declared SHA-256 that differs from the file",
            ),
            _ if voters.counts_by_area.is_empty() => ("no_counts", "no voters by area"),
            _ if voters
                .counts_by_area
                .keys()
                .any(|key| key.trim().is_empty()) =>
            {
                ("blank_area", "voters without an area")
            }
            _ => ("invalid_sha256", "an invalid SHA-256"),
        };
        findings.push(
            ReadinessFinding::new(
                VoterFileNotCertified,
                role,
                format!("The {name} event's last voter import has {explanation}"),
            )
            .value("reason", reason),
        );
    }
    let areas: BTreeSet<&String> = voters
        .counts_by_area
        .keys()
        .chain(event.current_voter_counts.keys())
        .collect();
    for area in areas {
        let imported = voters.counts_by_area.get(area).copied().unwrap_or(0);
        let current = event.current_voter_counts.get(area).copied().unwrap_or(0);
        if imported != current
            || voters.counts_by_area.contains_key(area)
                != event.current_voter_counts.contains_key(area)
        {
            findings.push(
                ReadinessFinding::new(
                    RegistryCountChanged,
                    role,
                    format!(
                        "The {name} event's registry has {current} voters in {} but the certified import has {imported}",
                        area_label(area)
                    ),
                )
                .area(area)
                .value("imported", imported)
                .value("current", current),
            );
        }
    }
    match &event.configuration {
        Some(configuration) => {
            for area in voters
                .counts_by_area
                .keys()
                .filter(|area| !configuration.areas.contains(*area))
            {
                findings.push(
                    ReadinessFinding::new(
                        VoterAreaNotConfigured,
                        role,
                        format!(
                            "The {name} event's certified voters include {}, which is not in its signed content",
                            area_label(area)
                        ),
                    )
                    .area(area),
                );
            }
        }
        None => findings.push(ReadinessFinding::new(
            ConfigurationMissing,
            role,
            format!(
                "The {name} event has no signed configuration to check its voter areas against"
            ),
        )),
    }
    findings
}

fn voter_findings(official: &EventEvidence, tested: &EventEvidence) -> Vec<ReadinessFinding> {
    use FindingCode::*;
    let mut findings = certification_findings(official, EventRole::Official);
    findings.extend(certification_findings(tested, EventRole::Tested));
    if let (Some(left), Some(right)) = (&official.voters, &tested.voters) {
        if !left.sha256.eq_ignore_ascii_case(&right.sha256) {
            findings.push(
                ReadinessFinding::new(
                    VoterFileMismatch,
                    EventRole::Both,
                    format!(
                        "The certified voter files differ: SHA-256 {} on the official event, {} on the tested event",
                        left.sha256, right.sha256
                    ),
                )
                .value("official", &left.sha256)
                .value("tested", &right.sha256),
            );
        }
        let areas: BTreeSet<&String> = left
            .counts_by_area
            .keys()
            .chain(right.counts_by_area.keys())
            .collect();
        for area in areas {
            let (official_count, tested_count) = (
                left.counts_by_area.get(area),
                right.counts_by_area.get(area),
            );
            if official_count != tested_count {
                let official_count = official_count.copied().unwrap_or(0);
                let tested_count = tested_count.copied().unwrap_or(0);
                findings.push(
                    ReadinessFinding::new(
                        VoterCountMismatch,
                        EventRole::Both,
                        format!(
                            "The certified files differ in {}: {official_count} voters on the official event, {tested_count} on the tested event",
                            area_label(area)
                        ),
                    )
                    .area(area)
                    .value("official", official_count)
                    .value("tested", tested_count),
                );
            }
        }
    }
    findings
}

/// Every channel that is no longer NOT_STARTED or has recorded voting dates.
fn started_channels(
    status: &ElectionStatus,
    role: EventRole,
    scope: &str,
    election_id: Option<&str>,
) -> Vec<ReadinessFinding> {
    CHANNELS
        .into_iter()
        .filter_map(|channel| {
            let current = status.status_by_channel(channel);
            let dated = !no_dates(status.dates_by_channel(channel));
            if current == VotingStatus::NOT_STARTED && !dated {
                return None;
            }
            let channel_name: &'static str = channel.into();
            let status_name: &'static str = current.into();
            let message = if current == VotingStatus::NOT_STARTED {
                format!("{scope}: the {channel_name} channel has recorded voting dates")
            } else {
                format!("{scope}: the {channel_name} channel is {status_name}")
            };
            let mut finding = ReadinessFinding::new(FindingCode::VotingStarted, role, message)
                .value("channel", channel_name)
                .value("status", status_name);
            if dated {
                finding = finding.value("dates", "recorded");
            }
            if let Some(id) = election_id {
                finding = finding.election(id);
            }
            Some(finding)
        })
        .collect()
}

fn official_zero_findings(official: &EventEvidence) -> Vec<ReadinessFinding> {
    use FindingCode::*;
    let role = EventRole::Official;
    let mut findings = Vec::new();
    for (code, count, what) in [
        (BallotsPresent, official.ballot_count, "stored ballots"),
        (
            ReceivedBallotsPresent,
            official.received_ballot_count,
            "ballots received for review",
        ),
        (
            TallySessionsPresent,
            official.tally_session_count,
            "tally sessions",
        ),
    ] {
        if count > 0 {
            findings.push(
                ReadinessFinding::new(code, role, format!("The official event has {count} {what}"))
                    .value("count", count),
            );
        }
    }
    match &official.status {
        Some(status) => findings.extend(started_channels(status, role, "The official event", None)),
        None => findings.push(ReadinessFinding::new(
            StatusMissing,
            role,
            "The official event has no voting status",
        )),
    }
    if official.elections.is_empty() {
        findings.push(ReadinessFinding::new(
            NoElections,
            role,
            "The official event has no elections",
        ));
    }
    for election in &official.elections {
        let scope = format!("Election {}", election.election_id);
        match &election.status {
            Some(status) => findings.extend(started_channels(
                status,
                role,
                &scope,
                Some(&election.election_id),
            )),
            None => findings.push(
                ReadinessFinding::new(StatusMissing, role, format!("{scope} has no voting status"))
                    .election(&election.election_id),
            ),
        }
    }
    findings
}

fn publication_findings(event: &EventEvidence, role: EventRole) -> Vec<ReadinessFinding> {
    use FindingCode::*;
    let name = role_name(role);
    let Some(publication) = &event.publication else {
        return vec![ReadinessFinding::new(
            PublicationMissing,
            role,
            format!("The {name} event has no active publication"),
        )];
    };
    // The design count only counts designs checked against the signed
    // manifest: none means they could not be checked at all.
    if publication.design_count == 0 {
        return vec![ReadinessFinding::new(
            PublicationEmpty,
            role,
            format!(
                "The {name} event's published ballot designs could not be checked against a signed manifest"
            ),
        )
        .value("publication_id", &publication.publication_id)];
    }
    let mut findings = Vec::new();
    if !publication.matches_manifest {
        findings.push(
            ReadinessFinding::new(
                PublicationMismatch,
                role,
                format!(
                    "The {name} event's published ballot designs ({}) do not match every design of its signed manifest",
                    publication.design_count
                ),
            )
            .value("publication_id", &publication.publication_id)
            .value("design_count", publication.design_count),
        );
    }
    findings
}

fn tested_finished_findings(tested: &EventEvidence) -> Vec<ReadinessFinding> {
    use FindingCode::*;
    let role = EventRole::Tested;
    let mut findings = publication_findings(tested, role);
    if tested.elections.is_empty() {
        findings.push(ReadinessFinding::new(
            NoElections,
            role,
            "The tested event has no elections",
        ));
    }
    if !tested.all_channels_closed {
        findings.push(ReadinessFinding::new(
            ChannelsOpen,
            role,
            "The tested event has an election whose voting channels have not all closed",
        ));
    }
    if !tested.seal_at_close {
        findings.push(ReadinessFinding::new(
            SealAtCloseOff,
            role,
            "The tested event does not seal ballot boxes at close",
        ));
    }
    if tested.expected_boxes.is_empty() {
        findings.push(ReadinessFinding::new(
            NoExpectedBoxes,
            role,
            "The tested event's publication expects no ballot boxes",
        ));
    }
    let mut unsealed: BTreeMap<&str, usize> = BTreeMap::new();
    for (election, _) in tested.expected_boxes.difference(&tested.sealed_boxes) {
        *unsealed.entry(election).or_default() += 1;
    }
    for (election, count) in unsealed {
        findings.push(
            ReadinessFinding::new(
                BoxesUnsealed,
                role,
                format!(
                    "Election {election} of the tested event has {count} unsealed ballot boxes"
                ),
            )
            .election(election)
            .value("unsealed", count),
        );
    }
    findings
}

pub fn evaluate(evidence: ReadinessEvidence, checked_at: String) -> ReadinessReport {
    let official = &evidence.official;
    let tested = &evidence.tested;
    // Each check passes exactly when it observes no failed predicate.
    let facts = [
        (CheckKind::Configuration, configuration_findings(official, tested),
            "Both events have the same signed content and verified approvers"),
        (CheckKind::CertifiedVoters, voter_findings(official, tested),
            "Both events have the same certified file, full current registry counts per area and signed import evidence"),
        (CheckKind::OfficialZero, official_zero_findings(official),
            "The official event has never opened voting and holds no ballots or tally sessions"),
        (CheckKind::Publication, publication_findings(official, EventRole::Official),
            "The complete official publication matches its signed manifest"),
        (CheckKind::TestedFinished, tested_finished_findings(tested),
            "The complete tested publication matches its signed manifest, every voting channel is closed and every expected ballot box is sealed"),
    ];
    let checks: Vec<ReadinessCheck> = facts
        .into_iter()
        .map(|(kind, findings, message)| ReadinessCheck {
            kind,
            passed: findings.is_empty(),
            message: message.into(),
            findings,
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
                "approvers": ["cert-a", "cert-b"], "areas": ["Area A", "Area B"]
            },
            "voters": {
                "import_id": format!("import-{id}"), "document_id": "document",
                "file_name": "certified.csv", "sha256": "ef".repeat(32),
                "declared_sha256": "ef".repeat(32),
                "counts_by_area": {"Area A": 3, "Area B": 2},
                "imported_at": "2026-10-14T09:00:00Z"
            },
            "voter_log_delivered": true,
            "current_voter_counts": {"Area A": 3, "Area B": 2},
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
    fn legacy_readiness_evidence_reads_and_only_emits_neutral_area_names() {
        let mut legacy = evidence();
        for event in ["official", "tested"] {
            let configuration = legacy[event]["configuration"].as_object_mut().unwrap();
            let areas = configuration
                .remove("areas")
                .or_else(|| configuration.remove("posts"))
                .unwrap();
            configuration.insert("posts".into(), areas);
            let voters = legacy[event]["voters"].as_object_mut().unwrap();
            let counts = voters
                .remove("counts_by_area")
                .or_else(|| voters.remove("counts_by_post"))
                .unwrap();
            voters.insert("counts_by_post".into(), counts);
        }
        let report = check(legacy.clone());
        assert!(report.passed);
        let mut stored_report = serde_json::to_value(&report).unwrap();
        stored_report["evidence"] = legacy;
        let restored: ReadinessReport = serde_json::from_value(stored_report).unwrap();
        let neutral = serde_json::to_value(&restored).unwrap();
        for event in ["official", "tested"] {
            let evidence = &neutral["evidence"][event];
            assert!(evidence["configuration"].get("areas").is_some());
            assert!(evidence["configuration"].get("posts").is_none());
            assert!(evidence["voters"].get("counts_by_area").is_some());
            assert!(evidence["voters"].get("counts_by_post").is_none());
        }
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
                "/tested/voters/counts_by_area/Area A",
                json!(4),
                CheckKind::CertifiedVoters,
            ),
            (
                "/tested/voter_log_delivered",
                json!(false),
                CheckKind::CertifiedVoters,
            ),
            (
                "/official/current_voter_counts/Area A",
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

    fn findings(report: &ReadinessReport, kind: CheckKind) -> Vec<ReadinessFinding> {
        report
            .checks
            .iter()
            .find(|check| check.kind == kind)
            .unwrap()
            .findings
            .clone()
    }

    #[test]
    fn each_failed_check_says_what_was_observed_on_which_event() {
        use EventRole::*;
        use FindingCode::*;
        let control = check(evidence());
        let cases = [
            (
                "/tested/configuration/content_sha256",
                json!("00".repeat(32)),
                CheckKind::Configuration,
                ContentMismatch,
                Both,
            ),
            (
                "/tested/configuration/approvers",
                json!(["other"]),
                CheckKind::Configuration,
                ApproversMismatch,
                Both,
            ),
            (
                "/tested/configuration/external_id",
                json!("other"),
                CheckKind::Configuration,
                ExternalIdMismatch,
                Both,
            ),
            (
                "/tested/configuration/areas",
                json!(["Area A"]),
                CheckKind::Configuration,
                AreasMismatch,
                Official,
            ),
            (
                "/tested/configuration",
                Value::Null,
                CheckKind::Configuration,
                ConfigurationMissing,
                Tested,
            ),
            (
                "/official/configuration/approvers",
                json!([]),
                CheckKind::Configuration,
                ConfigurationIncomplete,
                Official,
            ),
            (
                "/tested/voters/sha256",
                json!("00".repeat(32)),
                CheckKind::CertifiedVoters,
                VoterFileNotCertified,
                Tested,
            ),
            (
                "/tested/voters/sha256",
                json!("00".repeat(32)),
                CheckKind::CertifiedVoters,
                VoterFileMismatch,
                Both,
            ),
            (
                "/tested/voters/counts_by_area/Area A",
                json!(4),
                CheckKind::CertifiedVoters,
                VoterCountMismatch,
                Both,
            ),
            (
                "/tested/voter_log_delivered",
                json!(false),
                CheckKind::CertifiedVoters,
                VoterReceiptNotSigned,
                Tested,
            ),
            (
                "/official/current_voter_counts/Area A",
                json!(4),
                CheckKind::CertifiedVoters,
                RegistryCountChanged,
                Official,
            ),
            (
                "/tested/voter_import_in_progress",
                json!(true),
                CheckKind::CertifiedVoters,
                VoterImportInProgress,
                Tested,
            ),
            (
                "/official/voters",
                Value::Null,
                CheckKind::CertifiedVoters,
                VoterImportMissing,
                Official,
            ),
            (
                "/official/ballot_count",
                json!(1),
                CheckKind::OfficialZero,
                BallotsPresent,
                Official,
            ),
            (
                "/official/received_ballot_count",
                json!(1),
                CheckKind::OfficialZero,
                ReceivedBallotsPresent,
                Official,
            ),
            (
                "/official/tally_session_count",
                json!(1),
                CheckKind::OfficialZero,
                TallySessionsPresent,
                Official,
            ),
            (
                "/official/status/voting_status",
                json!("OPEN"),
                CheckKind::OfficialZero,
                VotingStarted,
                Official,
            ),
            (
                "/official/status",
                Value::Null,
                CheckKind::OfficialZero,
                StatusMissing,
                Official,
            ),
            (
                "/official/elections",
                json!([]),
                CheckKind::OfficialZero,
                NoElections,
                Official,
            ),
            (
                "/official/publication/matches_manifest",
                json!(false),
                CheckKind::Publication,
                PublicationMismatch,
                Official,
            ),
            (
                "/official/publication/design_count",
                json!(0),
                CheckKind::Publication,
                PublicationEmpty,
                Official,
            ),
            (
                "/official/publication",
                Value::Null,
                CheckKind::Publication,
                PublicationMissing,
                Official,
            ),
            (
                "/tested/publication",
                Value::Null,
                CheckKind::TestedFinished,
                PublicationMissing,
                Tested,
            ),
            (
                "/tested/all_channels_closed",
                json!(false),
                CheckKind::TestedFinished,
                ChannelsOpen,
                Tested,
            ),
            (
                "/tested/sealed_boxes",
                json!([]),
                CheckKind::TestedFinished,
                BoxesUnsealed,
                Tested,
            ),
            (
                "/tested/seal_at_close",
                json!(false),
                CheckKind::TestedFinished,
                SealAtCloseOff,
                Tested,
            ),
            (
                "/tested/expected_boxes",
                json!([]),
                CheckKind::TestedFinished,
                NoExpectedBoxes,
                Tested,
            ),
            (
                "/tested/elections",
                json!([]),
                CheckKind::TestedFinished,
                NoElections,
                Tested,
            ),
        ];
        for (path, replacement, kind, code, role) in cases {
            let mut value = evidence();
            let (parent, key) = path.rsplit_once('/').unwrap();
            value.pointer_mut(parent).unwrap()[key] = replacement;
            let report = check(value);
            assert!(!report.passed, "{path}");
            for (checked, expected) in report.checks.iter().zip(&control.checks) {
                // The condition stays the stable backend message.
                assert_eq!(checked.message, expected.message, "{path}");
                assert_eq!(checked.passed, checked.findings.is_empty(), "{path}");
                for finding in &checked.findings {
                    assert!(!finding.message.trim().is_empty(), "{path}");
                }
            }
            assert!(
                findings(&report, kind)
                    .iter()
                    .any(|finding| finding.code == code && finding.event == role),
                "{path}: {:#?}",
                findings(&report, kind)
            );
        }
    }

    #[test]
    fn diagnostics_name_the_affected_area_election_and_values() {
        let mut value = evidence();
        value["official"]["current_voter_counts"]["Area A"] = json!(4);
        let report = check(value);
        let changed = findings(&report, CheckKind::CertifiedVoters);
        let changed = changed
            .iter()
            .find(|finding| finding.code == FindingCode::RegistryCountChanged)
            .unwrap();
        assert_eq!(changed.area.as_deref(), Some("Area A"));
        assert_eq!(changed.values["imported"], "3");
        assert_eq!(changed.values["current"], "4");
        assert!(changed.message.contains("Area A"), "{}", changed.message);

        let mut value = evidence();
        value["official"]["elections"][0]["status"] = json!({"voting_status": "OPEN"});
        let report = check(value);
        let started = findings(&report, CheckKind::OfficialZero);
        assert_eq!(started.len(), 1, "{started:#?}");
        assert_eq!(started[0].election_id.as_deref(), Some("election-official"));
        assert_eq!(started[0].values["channel"], "ONLINE");
        assert_eq!(started[0].values["status"], "OPEN");

        let mut value = evidence();
        value["official"]["elections"][0]["status"] = json!({
            "voting_period_dates": {"first_started_at": "2026-10-13T10:00:00Z"}
        });
        let report = check(value);
        let dated = findings(&report, CheckKind::OfficialZero);
        assert_eq!(dated[0].code, FindingCode::VotingStarted);
        assert_eq!(dated[0].values["dates"], "recorded");

        let mut value = evidence();
        value["tested"]["sealed_boxes"] = json!([]);
        let report = check(value);
        let unsealed = findings(&report, CheckKind::TestedFinished);
        assert_eq!(unsealed[0].election_id.as_deref(), Some("election-tested"));
        assert_eq!(unsealed[0].values["unsealed"], "1");
    }

    #[test]
    fn passing_reports_keep_the_signed_check_shape() {
        let report = serde_json::to_value(check(evidence())).unwrap();
        for check in report["checks"].as_array().unwrap() {
            let mut keys: Vec<_> = check.as_object().unwrap().keys().cloned().collect();
            keys.sort();
            assert_eq!(keys, ["kind", "message", "passed"]);
        }
    }

    #[test]
    fn reports_written_before_diagnostics_still_read() {
        let mut value = evidence();
        value["official"]["ballot_count"] = json!(1);
        let mut stored = serde_json::to_value(check(value)).unwrap();
        for check in stored["checks"].as_array_mut().unwrap() {
            check.as_object_mut().unwrap().remove("findings");
        }
        let restored: ReadinessReport = serde_json::from_value(stored).unwrap();
        assert!(!restored.passed);
        assert!(restored
            .checks
            .iter()
            .all(|check| check.findings.is_empty()));
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
