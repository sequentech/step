// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Read-only rehearsal comparisons between two separately imported events.
//!
//! A rehearsal report is never a readiness verdict. It has no `passed` flag,
//! no `log_entry_id` and no production variant, so neither the readiness
//! recorder nor the signed `ReadinessVerified` writer can accept it. Each event
//! must prove its own certified census; the censuses are never compared for
//! equality, and publications are never compared across events.

use super::readiness::{
    area_label, certification_findings, finished_findings, official_zero_findings, role_name,
    ConfigurationEvidence, EventEvidence, EventRole, FindingCode, ReadinessFinding,
    RehearsalComparison,
};
use serde::{Deserialize, Serialize};

/// The independent questions a rehearsal comparison answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RehearsalCheckKind {
    /// The same approved signed content and approvers, separately imported.
    Configuration,
    /// Each event's own certified census evidence, never cross-compared.
    OwnCensus,
    /// Official/STL only: whether the STL census is a certified subset.
    CensusSubset,
    /// The official event is untouched; each rehearsal event has closed.
    Lifecycle,
    /// Each rehearsal event's own publication and ballot-box seals.
    Seal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RehearsalVerdict {
    Consistent,
    Inconsistent,
    /// The available evidence cannot prove the claim either way.
    NotVerified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalCheck {
    pub kind: RehearsalCheckKind,
    pub verdict: RehearsalVerdict,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<ReadinessFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleEvidence {
    pub role: EventRole,
    pub evidence: EventEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalReport {
    /// The explicit read-only discriminant, the same value the request named.
    pub comparison: RehearsalComparison,
    pub verdict: RehearsalVerdict,
    pub checked_at: String,
    pub checks: Vec<RehearsalCheck>,
    /// The event in the request's first position (`official_event_id`).
    pub reference: RoleEvidence,
    /// The event in the request's second position (`tested_event_id`).
    pub compared: RoleEvidence,
}

/// The proofs that would be needed to certify an STL census as a subset of
/// the official one, none of which the certified evidence records.
pub const SUBSET_MISSING_PROOF: &str =
    "stable_voter_identity,certified_membership,approved_selection";

fn configuration_complete(
    configuration: &ConfigurationEvidence,
    role: EventRole,
) -> Vec<ReadinessFinding> {
    [
        ("external_id", configuration.external_id.is_empty()),
        ("approvers", configuration.approvers.is_empty()),
        ("areas", configuration.areas.is_empty()),
        ("content_sha256", configuration.content_sha256.len() != 64),
    ]
    .into_iter()
    .filter(|(_, missing)| *missing)
    .map(|(field, _)| {
        ReadinessFinding::new(
            FindingCode::ConfigurationIncomplete,
            role,
            format!(
                "The {} event's signed configuration has no valid {field}",
                role_name(role)
            ),
        )
        .value("field", field)
    })
    .collect()
}

/// Approved content, external ID, approvers and areas must match; the two
/// imports must be separate revisions. Manifest hashes may differ.
fn configuration_findings(
    left: &EventEvidence,
    left_role: EventRole,
    right: &EventEvidence,
    right_role: EventRole,
) -> Vec<ReadinessFinding> {
    use FindingCode::*;
    let (left_name, right_name) = (role_name(left_role), role_name(right_role));
    let mut findings = Vec::new();
    if left.event_id == right.event_id {
        findings.push(ReadinessFinding::new(
            SameEvent,
            EventRole::Both,
            format!("The {left_name} and {right_name} events are the same event"),
        ));
    }
    let (a, b) = match (&left.configuration, &right.configuration) {
        (Some(a), Some(b)) => (a, b),
        (a, b) => {
            for (role, configuration) in [(left_role, a.is_none()), (right_role, b.is_none())] {
                if configuration {
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
    findings.extend(configuration_complete(a, left_role));
    findings.extend(configuration_complete(b, right_role));
    if a.revision == b.revision {
        findings.push(
            ReadinessFinding::new(
                RevisionNotDistinct,
                EventRole::Both,
                format!(
                    "Both events carry package revision {}; each event needs its own separately imported revision",
                    a.revision
                ),
            )
            .value("revision", a.revision),
        );
    }
    if a.external_id != b.external_id {
        findings.push(
            ReadinessFinding::new(
                ExternalIdMismatch,
                EventRole::Both,
                format!(
                    "The configurations differ: the {left_name} event is {} and the {right_name} event is {}",
                    a.external_id, b.external_id
                ),
            )
            .value("left", &a.external_id)
            .value("right", &b.external_id),
        );
    }
    if a.content_sha256 != b.content_sha256 {
        findings.push(
            ReadinessFinding::new(
                ContentMismatch,
                EventRole::Both,
                format!(
                    "The signed content differs: SHA-256 {} on the {left_name} event, {} on the {right_name} event",
                    a.content_sha256, b.content_sha256
                ),
            )
            .value("left", &a.content_sha256)
            .value("right", &b.content_sha256),
        );
    }
    if a.approvers != b.approvers {
        let only_left = a.approvers.difference(&b.approvers).count();
        let only_right = b.approvers.difference(&a.approvers).count();
        findings.push(
            ReadinessFinding::new(
                ApproversMismatch,
                EventRole::Both,
                format!(
                    "The verified approvers differ: {only_left} only on the {left_name} event, {only_right} only on the {right_name} event"
                ),
            )
            .value("only_left", only_left)
            .value("only_right", only_right),
        );
    }
    for (role, area) in a
        .areas
        .difference(&b.areas)
        .map(|area| (left_role, area))
        .chain(b.areas.difference(&a.areas).map(|area| (right_role, area)))
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

/// Area counts can refute a subset; nothing in the certified evidence can
/// prove one. The result is never consistent.
fn subset_findings(official: &EventEvidence, stl: &EventEvidence) -> Vec<ReadinessFinding> {
    use FindingCode::*;
    let mut findings = Vec::new();
    if let (Some(official_voters), Some(stl_voters)) = (&official.voters, &stl.voters) {
        for (area, count) in &stl_voters.counts_by_area {
            let available = official_voters
                .counts_by_area
                .get(area)
                .copied()
                .unwrap_or(0);
            if *count > available {
                findings.push(
                    ReadinessFinding::new(
                        SubsetAreaExceeds,
                        EventRole::Stl,
                        format!(
                            "The STL census has {count} voters in {}, more than the {available} of the official certified census",
                            area_label(area)
                        ),
                    )
                    .area(area)
                    .value("stl", count)
                    .value("official", available),
                );
            }
        }
    }
    if findings.is_empty() {
        findings.push(
            ReadinessFinding::new(
                SubsetNotVerified,
                EventRole::Both,
                "Subset not verified: each certified voter import records only the file SHA-256, the declared SHA-256 and voter counts by area. No stable voter identity is bound to the certified official file, there is no certified membership record and no approved selection of the STL voters, and voter account IDs (and sometimes usernames) are generated separately in each event, so no join can prove that every STL voter is an official voter",
            )
            .value("missing", SUBSET_MISSING_PROOF),
        );
    }
    findings
}

fn is_lifecycle(code: FindingCode) -> bool {
    matches!(code, FindingCode::NoElections | FindingCode::ChannelsOpen)
}

/// The official event must be untouched; a rehearsal event must have closed.
/// The production zero check is never applied to a rehearsal event.
fn lifecycle_findings(event: &EventEvidence, role: EventRole) -> Vec<ReadinessFinding> {
    match role {
        EventRole::Official => official_zero_findings(event),
        _ => finished_findings(event, role)
            .into_iter()
            .filter(|finding| is_lifecycle(finding.code))
            .collect(),
    }
}

/// Each rehearsal event's own publication against its own signed manifest,
/// and its own expected ballot boxes sealed. The official event has none.
fn seal_findings(event: &EventEvidence, role: EventRole) -> Vec<ReadinessFinding> {
    match role {
        EventRole::Official => Vec::new(),
        _ => finished_findings(event, role)
            .into_iter()
            .filter(|finding| !is_lifecycle(finding.code))
            .collect(),
    }
}

fn lifecycle_message(comparison: RehearsalComparison) -> &'static str {
    match comparison {
        RehearsalComparison::ErtStl => {
            "The ERT and STL events each have elections and every voting channel closed"
        }
        RehearsalComparison::OfficialErt => {
            "The official event has never opened voting and holds no ballots or tally sessions; the ERT event has elections and every voting channel closed"
        }
        RehearsalComparison::OfficialStl => {
            "The official event has never opened voting and holds no ballots or tally sessions; the STL event has elections and every voting channel closed"
        }
    }
}

fn verdict_of(kind: RehearsalCheckKind, findings: &[ReadinessFinding]) -> RehearsalVerdict {
    if findings.is_empty() {
        RehearsalVerdict::Consistent
    } else if kind == RehearsalCheckKind::CensusSubset
        && findings
            .iter()
            .all(|finding| finding.code == FindingCode::SubsetNotVerified)
    {
        RehearsalVerdict::NotVerified
    } else {
        RehearsalVerdict::Inconsistent
    }
}

pub fn evaluate_rehearsal(
    comparison: RehearsalComparison,
    reference: EventEvidence,
    compared: EventEvidence,
    checked_at: String,
) -> RehearsalReport {
    use RehearsalCheckKind::*;
    let (left_role, right_role) = comparison.roles();
    let (left, right) = (&reference, &compared);
    let mut census = certification_findings(left, left_role);
    census.extend(certification_findings(right, right_role));
    let mut lifecycle = lifecycle_findings(left, left_role);
    lifecycle.extend(lifecycle_findings(right, right_role));
    let mut seal = seal_findings(left, left_role);
    seal.extend(seal_findings(right, right_role));
    let mut facts = vec![
        (
            Configuration,
            configuration_findings(left, left_role, right, right_role),
            "Both events have the same approved signed content, external ID, verified approvers and areas, from separately imported package revisions",
        ),
        (
            OwnCensus,
            census,
            "Each event has its own certified voter import, signed import receipt and matching current registry; the two censuses are not compared",
        ),
    ];
    if comparison == RehearsalComparison::OfficialStl {
        facts.push((
            CensusSubset,
            subset_findings(left, right),
            "The STL census is a certified subset of the official certified census",
        ));
    }
    facts.push((Lifecycle, lifecycle, lifecycle_message(comparison)));
    facts.push((
        Seal,
        seal,
        "Each rehearsal event's complete publication matches its own signed manifest and every expected ballot box is sealed",
    ));
    let checks: Vec<RehearsalCheck> = facts
        .into_iter()
        .map(|(kind, findings, message)| RehearsalCheck {
            kind,
            verdict: verdict_of(kind, &findings),
            message: message.into(),
            findings,
        })
        .collect();
    let any = |verdict: RehearsalVerdict| checks.iter().any(|check| check.verdict == verdict);
    let verdict = if any(RehearsalVerdict::Inconsistent) {
        RehearsalVerdict::Inconsistent
    } else if any(RehearsalVerdict::NotVerified) {
        RehearsalVerdict::NotVerified
    } else {
        RehearsalVerdict::Consistent
    };
    RehearsalReport {
        comparison,
        verdict,
        checked_at,
        checks,
        reference: RoleEvidence {
            role: left_role,
            evidence: reference,
        },
        compared: RoleEvidence {
            role: right_role,
            evidence: compared,
        },
    }
}

#[cfg(test)]
#[path = "../../tests/support/readiness_rehearsal_tests.rs"]
mod tests;
