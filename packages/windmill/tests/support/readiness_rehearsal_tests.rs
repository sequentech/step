// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Read-only rehearsal comparisons over generated synthetic signed lineage:
//! three events with distinct UUIDs, monotonic package revisions, the same
//! approved content, approvers and areas, and each its own certified census.

use super::*;
use crate::domain::readiness::{ComparisonMode, ReadinessReport};
use sequent_core::election_config::design::{BallotDesign, DesignDigestAlgorithm};
use sequent_core::election_config::manifest::Content;
use sequent_core::election_config::package_verify::CertificateIdentity;
use serde_json::{json, Value};
use uuid::Uuid;

const CHECKED_AT: &str = "2026-10-14T10:00:00Z";

/// The approved content every event imports, digested by the real producer.
fn approved_content(areas: &[&str]) -> Content {
    Content {
        files: vec![],
        ballot_design_digest_algorithm: DesignDigestAlgorithm::V1,
        ballot_designs: areas
            .iter()
            .map(|area| BallotDesign {
                area: (*area).into(),
                election: "approved-election".into(),
                sha256: sequent_core::election_config::manifest::sha256_hex(area.as_bytes()),
                version: 1,
            })
            .collect(),
        reports: vec![],
    }
}

/// Approver identities serialized exactly as the readiness snapshot does.
fn approvers(names: &[&str]) -> Vec<String> {
    names
        .iter()
        .map(|name| {
            let identity: CertificateIdentity = serde_json::from_value(json!({
                "subject": format!("CN={name}"), "issuer": "CN=Synthetic CA",
                "serial": "01", "fingerprint_sha256": sequent_core::election_config::manifest::sha256_hex(name.as_bytes())
            }))
            .unwrap();
            serde_json::to_string(&identity).unwrap()
        })
        .collect()
}

/// One event of the lineage: its own UUID, revision and certified census, in
/// either an untouched (official) or closed and sealed (rehearsal) state.
fn event(revision: u64, census: &[(&str, u64)], census_sha: &str, closed: bool) -> Value {
    let id = Uuid::new_v4().to_string();
    let election = Uuid::new_v4().to_string();
    let content = approved_content(&["Area A", "Area B"]);
    let counts: serde_json::Map<String, Value> = census
        .iter()
        .map(|(area, count)| ((*area).to_string(), json!(count)))
        .collect();
    let closed_status = json!({"voting_status": "CLOSED", "voting_period_dates": {
        "first_started_at": "2026-10-13T09:00:00Z", "last_stopped_at": "2026-10-13T10:00:00Z"
    }});
    let status = if closed { closed_status } else { json!({}) };
    let boxes: Vec<Value> = if closed {
        vec![json!([election, "Area A"]), json!([election, "Area B"])]
    } else {
        vec![]
    };
    json!({
        "event_id": id,
        "configuration": {
            "external_id": "approved-2028", "revision": revision,
            "content_sha256": content.sha256().unwrap(),
            "manifest_sha256": sequent_core::election_config::manifest::sha256_hex(format!("manifest-{revision}").as_bytes()),
            "approvers": approvers(&["Approver One", "Approver Two"]),
            "areas": ["Area A", "Area B"]
        },
        "voters": {
            "import_id": Uuid::new_v4().to_string(), "document_id": Uuid::new_v4().to_string(),
            "file_name": "synthetic.csv", "sha256": census_sha, "declared_sha256": census_sha,
            "counts_by_area": counts, "imported_at": "2026-10-12T09:00:00Z"
        },
        "voter_log_delivered": true,
        "current_voter_counts": counts,
        "voter_import_in_progress": false,
        "status": status,
        "elections": [{"election_id": election, "status": status}],
        "ballot_count": if closed { 3 } else { 0 },
        "received_ballot_count": 0,
        "tally_session_count": if closed { 1 } else { 0 },
        "publication": if closed {
            json!({"publication_id": Uuid::new_v4().to_string(), "matches_manifest": true, "design_count": 2})
        } else { Value::Null },
        "seal_at_close": closed,
        "expected_boxes": boxes,
        "sealed_boxes": boxes,
        "all_channels_closed": closed,
        "locked_down": false
    })
}

/// Official (revision 8, full certified census), ERT (revision 9, its own
/// certified test census) and STL (revision 10, a selected census).
fn lineage() -> (Value, Value, Value) {
    (
        event(8, &[("Area A", 3), ("Area B", 2)], &"0a".repeat(32), false),
        event(9, &[("Area A", 7), ("Area B", 4)], &"0b".repeat(32), true),
        event(10, &[("Area A", 1), ("Area B", 1)], &"0c".repeat(32), true),
    )
}

fn pair(comparison: RehearsalComparison) -> (Value, Value) {
    let (official, ert, stl) = lineage();
    match comparison {
        RehearsalComparison::ErtStl => (ert, stl),
        RehearsalComparison::OfficialErt => (official, ert),
        RehearsalComparison::OfficialStl => (official, stl),
    }
}

fn compare(comparison: RehearsalComparison, reference: Value, compared: Value) -> RehearsalReport {
    evaluate_rehearsal(
        comparison,
        serde_json::from_value(reference).unwrap(),
        serde_json::from_value(compared).unwrap(),
        CHECKED_AT.into(),
    )
}

fn check(report: &RehearsalReport, kind: RehearsalCheckKind) -> &RehearsalCheck {
    report
        .checks
        .iter()
        .find(|check| check.kind == kind)
        .unwrap_or_else(|| panic!("{kind:?} missing: {:#?}", report.checks))
}

const ALL: [RehearsalComparison; 3] = [
    RehearsalComparison::ErtStl,
    RehearsalComparison::OfficialErt,
    RehearsalComparison::OfficialStl,
];

#[test]
fn each_mode_reports_its_roles_and_only_official_stl_is_never_consistent() {
    use RehearsalCheckKind::*;
    for comparison in ALL {
        let (reference, compared) = pair(comparison);
        let report = compare(comparison, reference.clone(), compared.clone());
        let (left, right) = comparison.roles();
        assert_eq!(report.reference.role, left);
        assert_eq!(report.compared.role, right);
        assert_eq!(report.reference.evidence.event_id, reference["event_id"]);
        assert_eq!(report.compared.evidence.event_id, compared["event_id"]);
        assert_ne!(
            report.reference.evidence.event_id,
            report.compared.evidence.event_id
        );
        let kinds: Vec<_> = report.checks.iter().map(|check| check.kind).collect();
        for kind in [Configuration, OwnCensus, Lifecycle, Seal] {
            assert_eq!(check(&report, kind).verdict, RehearsalVerdict::Consistent, "{comparison:?} {kind:?}: {:#?}", report.checks);
        }
        if comparison == RehearsalComparison::OfficialStl {
            assert_eq!(kinds, [Configuration, OwnCensus, CensusSubset, Lifecycle, Seal]);
            assert_eq!(report.verdict, RehearsalVerdict::NotVerified);
            let subset = check(&report, CensusSubset);
            assert_eq!(subset.verdict, RehearsalVerdict::NotVerified);
            assert_eq!(subset.findings.len(), 1);
            assert_eq!(subset.findings[0].code, FindingCode::SubsetNotVerified);
            assert_eq!(subset.findings[0].values["missing"], SUBSET_MISSING_PROOF);
            for proof in ["stable_voter_identity", "certified_membership", "approved_selection"] {
                assert!(SUBSET_MISSING_PROOF.split(',').any(|missing| missing == proof));
            }
        } else {
            assert_eq!(kinds, [Configuration, OwnCensus, Lifecycle, Seal]);
            assert_eq!(report.verdict, RehearsalVerdict::Consistent);
        }
    }
}

#[test]
fn certified_censuses_are_never_compared_for_equality() {
    // The lineage's three certified files and counts all differ.
    for comparison in [RehearsalComparison::ErtStl, RehearsalComparison::OfficialErt] {
        let (reference, compared) = pair(comparison);
        assert_ne!(reference["voters"]["sha256"], compared["voters"]["sha256"]);
        assert_ne!(
            reference["voters"]["counts_by_area"],
            compared["voters"]["counts_by_area"]
        );
        let report = compare(comparison, reference, compared);
        assert_eq!(report.verdict, RehearsalVerdict::Consistent, "{:#?}", report.checks);
        assert!(report.checks.iter().flat_map(|check| &check.findings).all(|finding| !matches!(
            finding.code,
            FindingCode::VoterFileMismatch | FindingCode::VoterCountMismatch
        )));
    }
}

#[test]
fn a_census_that_exceeds_the_official_one_is_proven_not_a_subset() {
    let (official, _, mut stl) = lineage();
    stl["voters"]["counts_by_area"]["Area B"] = json!(5);
    stl["current_voter_counts"]["Area B"] = json!(5);
    stl["voters"]["counts_by_area"]["Area Z"] = json!(1);
    stl["current_voter_counts"]["Area Z"] = json!(1);
    let report = compare(RehearsalComparison::OfficialStl, official, stl);
    let subset = check(&report, RehearsalCheckKind::CensusSubset);
    assert_eq!(subset.verdict, RehearsalVerdict::Inconsistent);
    assert_eq!(report.verdict, RehearsalVerdict::Inconsistent);
    let exceeded: Vec<_> = subset
        .findings
        .iter()
        .map(|finding| {
            assert_eq!(finding.code, FindingCode::SubsetAreaExceeds);
            assert_eq!(finding.event, EventRole::Stl);
            (finding.area.clone().unwrap(), finding.values["official"].clone())
        })
        .collect();
    assert_eq!(
        exceeded,
        [("Area B".to_string(), "2".to_string()), ("Area Z".into(), "0".into())]
    );
}

#[test]
fn equal_counts_still_cannot_verify_a_subset() {
    // Identical counts and even an identical file are not member evidence.
    let (official, _, mut stl) = lineage();
    stl["voters"]["counts_by_area"] = official["voters"]["counts_by_area"].clone();
    stl["current_voter_counts"] = official["current_voter_counts"].clone();
    stl["voters"]["sha256"] = official["voters"]["sha256"].clone();
    stl["voters"]["declared_sha256"] = official["voters"]["sha256"].clone();
    let report = compare(RehearsalComparison::OfficialStl, official, stl);
    assert_eq!(report.verdict, RehearsalVerdict::NotVerified);
}

#[test]
fn every_gap_refuses_on_the_event_and_check_where_it_was_observed() {
    use RehearsalCheckKind::*;
    for comparison in ALL {
        let (left, right) = comparison.roles();
        // (event side, JSON path, replacement, refusing check, code, role)
        let cases: Vec<(bool, &str, Value, RehearsalCheckKind, FindingCode, EventRole)> = vec![
            (true, "/configuration/content_sha256", json!("00".repeat(32)), Configuration, FindingCode::ContentMismatch, EventRole::Both),
            (true, "/configuration/approvers", json!(approvers(&["Someone Else"])), Configuration, FindingCode::ApproversMismatch, EventRole::Both),
            (true, "/configuration/external_id", json!("other-2028"), Configuration, FindingCode::ExternalIdMismatch, EventRole::Both),
            (true, "/configuration/areas", json!(["Area A"]), Configuration, FindingCode::AreasMismatch, left),
            (false, "/configuration/approvers", json!([]), Configuration, FindingCode::ConfigurationIncomplete, left),
            (true, "/configuration", Value::Null, Configuration, FindingCode::ConfigurationMissing, right),
            (true, "/voters", Value::Null, OwnCensus, FindingCode::VoterImportMissing, right),
            (false, "/voter_log_delivered", json!(false), OwnCensus, FindingCode::VoterReceiptNotSigned, left),
            (true, "/voters/declared_sha256", Value::Null, OwnCensus, FindingCode::VoterFileNotCertified, right),
            (true, "/current_voter_counts/Area A", json!(99), OwnCensus, FindingCode::RegistryCountChanged, right),
            (false, "/voter_import_in_progress", json!(true), OwnCensus, FindingCode::VoterImportInProgress, left),
            (true, "/all_channels_closed", json!(false), Lifecycle, FindingCode::ChannelsOpen, right),
            (true, "/sealed_boxes", json!([]), Seal, FindingCode::BoxesUnsealed, right),
            (true, "/seal_at_close", json!(false), Seal, FindingCode::SealAtCloseOff, right),
            (true, "/publication/matches_manifest", json!(false), Seal, FindingCode::PublicationMismatch, right),
        ];
        for (compared_side, path, replacement, kind, code, role) in cases {
            let (mut reference, mut compared) = pair(comparison);
            let target = if compared_side { &mut compared } else { &mut reference };
            let (parent, key) = path.rsplit_once('/').unwrap();
            let parent = if parent.is_empty() { &mut *target } else { target.pointer_mut(parent).unwrap() };
            parent[key] = replacement;
            let report = compare(comparison, reference, compared);
            assert_eq!(report.verdict, RehearsalVerdict::Inconsistent, "{comparison:?} {path}");
            let refused = check(&report, kind);
            assert_eq!(refused.verdict, RehearsalVerdict::Inconsistent, "{comparison:?} {path}");
            assert!(
                refused.findings.iter().any(|finding| finding.code == code && finding.event == role),
                "{comparison:?} {path}: {:#?}",
                refused.findings
            );
        }
    }
}

#[test]
fn separate_imports_need_distinct_revisions_and_events() {
    for comparison in ALL {
        let (reference, mut compared) = pair(comparison);
        compared["configuration"]["revision"] = reference["configuration"]["revision"].clone();
        let report = compare(comparison, reference.clone(), compared);
        let findings = &check(&report, RehearsalCheckKind::Configuration).findings;
        assert!(findings.iter().any(|finding| finding.code == FindingCode::RevisionNotDistinct));
        assert_eq!(report.verdict, RehearsalVerdict::Inconsistent);

        let report = compare(comparison, reference.clone(), reference);
        assert!(check(&report, RehearsalCheckKind::Configuration)
            .findings
            .iter()
            .any(|finding| finding.code == FindingCode::SameEvent));
    }
}

#[test]
fn lifecycle_direction_is_by_role_and_rehearsals_are_not_production_zero() {
    use RehearsalCheckKind::Lifecycle;
    // A closed, sealed rehearsal event with ballots and a tally is complete,
    // never refused by the production zero check (ERT↔STL has no official).
    let (_, ert, stl) = lineage();
    assert!(ert["ballot_count"].as_u64().unwrap() > 0);
    assert!(stl["tally_session_count"].as_u64().unwrap() > 0);
    let report = compare(RehearsalComparison::ErtStl, ert.clone(), stl.clone());
    assert_eq!(report.verdict, RehearsalVerdict::Consistent, "{:#?}", report.checks);
    assert!(report.checks.iter().flat_map(|check| &check.findings).all(|finding| finding.event != EventRole::Official));

    // The official event must be untouched: a used event in the official
    // position is refused, and an unopened event cannot stand in for ERT.
    let (official, ert, _) = lineage();
    let reversed = compare(RehearsalComparison::OfficialErt, ert.clone(), official.clone());
    let lifecycle = check(&reversed, Lifecycle);
    assert_eq!(lifecycle.verdict, RehearsalVerdict::Inconsistent);
    for code in [FindingCode::BallotsPresent, FindingCode::TallySessionsPresent, FindingCode::VotingStarted] {
        assert!(lifecycle.findings.iter().any(|finding| finding.code == code && finding.event == EventRole::Official), "{code:?}");
    }
    assert!(lifecycle.findings.iter().any(|finding| finding.code == FindingCode::ChannelsOpen && finding.event == EventRole::Ert));
    assert_eq!(check(&reversed, RehearsalCheckKind::Seal).verdict, RehearsalVerdict::Inconsistent);

    // Lifecycle and seals are separate questions with separate verdicts.
    let (official, mut ert, _) = lineage();
    ert["sealed_boxes"] = json!([]);
    let report = compare(RehearsalComparison::OfficialErt, official, ert);
    assert_eq!(check(&report, Lifecycle).verdict, RehearsalVerdict::Consistent);
    assert_eq!(check(&report, RehearsalCheckKind::Seal).verdict, RehearsalVerdict::Inconsistent);
}

#[test]
fn no_publication_is_compared_across_events() {
    // An untouched official event has no publication of its own to compare;
    // each rehearsal event only answers for its own publication.
    for comparison in [RehearsalComparison::OfficialErt, RehearsalComparison::OfficialStl] {
        let (official, compared) = pair(comparison);
        assert!(official["publication"].is_null());
        let report = compare(comparison, official, compared);
        assert!(report.checks.iter().flat_map(|check| &check.findings).all(|finding| !matches!(
            finding.code,
            FindingCode::PublicationMissing | FindingCode::PublicationEmpty
        )));
    }
    let (_, ert, mut stl) = lineage();
    stl["publication"]["publication_id"] = json!(Uuid::new_v4().to_string());
    stl["publication"]["design_count"] = json!(7);
    assert_eq!(compare(RehearsalComparison::ErtStl, ert, stl).verdict, RehearsalVerdict::Consistent);
}

#[test]
fn the_read_only_discriminant_survives_serialization_and_never_reads_as_a_signed_report() {
    for (comparison, wire) in [
        (RehearsalComparison::ErtStl, "rehearsal_ert_stl"),
        (RehearsalComparison::OfficialErt, "rehearsal_official_ert"),
        (RehearsalComparison::OfficialStl, "rehearsal_official_stl"),
    ] {
        let (reference, compared) = pair(comparison);
        let report = compare(comparison, reference, compared);
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["comparison"], wire);
        assert!(value.get("passed").is_none());
        assert!(value.get("log_entry_id").is_none());
        assert!(value.get("evidence").is_none());
        let mode: ComparisonMode = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(mode.rehearsal(), Some(comparison));
        // A rehearsal report is not a readiness report, so it can never be
        // handed to the recorder or the signed ReadinessVerified writer.
        assert!(serde_json::from_value::<ReadinessReport>(value.clone()).is_err());
        let restored: RehearsalReport = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&restored).unwrap(), value);
        // Injected signed-report fields are refused, not ignored.
        let mut forged = value.clone();
        forged["passed"] = json!(true);
        assert!(serde_json::from_value::<RehearsalReport>(forged).is_err());
        let mut forged = value;
        forged["log_entry_id"] = json!(Uuid::new_v4().to_string());
        assert!(serde_json::from_value::<RehearsalReport>(forged).is_err());
    }
    // Production has no rehearsal form, and a production report is not one.
    assert_eq!(ComparisonMode::default(), ComparisonMode::Production);
    assert_eq!(ComparisonMode::Production.rehearsal(), None);
    assert!(serde_json::from_value::<RehearsalComparison>(json!("production")).is_err());
    let production = crate::domain::readiness::evaluate(
        serde_json::from_value(crate::domain::readiness::tests::evidence()).unwrap(),
        CHECKED_AT.into(),
    );
    let production = serde_json::to_value(production).unwrap();
    assert!(production.get("comparison").is_none());
    assert!(serde_json::from_value::<RehearsalReport>(production).is_err());
}
