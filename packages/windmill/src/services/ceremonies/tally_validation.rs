// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

pub use crate::domain::tally_ceremony::TallyValidationError;
use crate::ports::tally_ceremony::BallotBoxSealState;
use crate::postgres::ballot_box_seal::BallotBoxSealStatus;
use crate::services::initialization_record::initialization_area_filter;
use sequent_core::ballot::{AllowTallyStatus, BallotBoxSealPolicy, ElectionStatus, InitReport};
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::{Election, TallySession, VotingChannels};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub fn validate_tally_elections(
    elections: &[Election],
    selected_ids: &[String],
    tally_type: TallyType,
) -> Result<(), TallyValidationError> {
    if selected_ids.is_empty() {
        return Err(TallyValidationError::new(
            "Select at least one election to tally.",
        ));
    }
    let mut seen = HashSet::new();
    for id in selected_ids {
        if !seen.insert(id) {
            return Err(TallyValidationError::new(
                "An election was selected more than once.",
            ));
        }
        let election = elections
            .iter()
            .find(|election| &election.id == id)
            .ok_or_else(|| {
                TallyValidationError::new(format!("Selected election {id} was not found."))
            })?;
        let status = election
            .status
            .as_ref()
            .and_then(|status| serde_json::from_value::<ElectionStatus>(status.clone()).ok())
            .ok_or_else(|| {
                TallyValidationError::new(format!("Election {id} has missing or invalid status."))
            })?;
        let reason = if status.is_published != Some(true) {
            Some("publish the election before creating its tally")
        } else {
            match tally_type {
                TallyType::INITIALIZATION_REPORT if status.init_report != InitReport::ALLOWED => {
                    Some("initialization reports are not allowed")
                }
                TallyType::INITIALIZATION_REPORT
                    if ![
                        &status.voting_status,
                        &status.kiosk_voting_status,
                        &status.early_voting_status,
                        &status.telephone_voting_status,
                    ]
                    .iter()
                    .all(|channel_status| channel_status.is_not_started()) =>
                {
                    Some("generate its initialization report before any voting channel opens")
                }
                TallyType::INITIALIZATION_REPORT => None,
                TallyType::ELECTORAL_RESULTS => match status.allow_tally {
                    AllowTallyStatus::ALLOWED => None,
                    AllowTallyStatus::DISALLOWED => {
                        Some("tallying is disabled by its configuration")
                    }
                    AllowTallyStatus::REQUIRES_VOTING_PERIOD_END => {
                        let channels = election
                            .voting_channels
                            .clone()
                            .map(serde_json::from_value::<VotingChannels>)
                            .transpose()
                            .map_err(|error| {
                                TallyValidationError::new(format!(
                                    "Election {id} has invalid voting_channels: {error}"
                                ))
                            })?;
                        let secondary_closed = match channels {
                            None => {
                                status.kiosk_voting_status.is_closed_or_never_started()
                                    && status.early_voting_status.is_closed_or_never_started()
                                    && status.telephone_voting_status.is_closed_or_never_started()
                            }
                            Some(channels) => {
                                (channels.kiosk != Some(true)
                                    || status.kiosk_voting_status.is_closed_or_never_started())
                                    && (channels.early_voting != Some(true)
                                        || status.early_voting_status.is_closed_or_never_started())
                                    && (channels.telephone != Some(true)
                                        || status
                                            .telephone_voting_status
                                            .is_closed_or_never_started())
                            }
                        };
                        if status.voting_status.is_closed() && secondary_closed {
                            None
                        } else {
                            Some("end its voting period and stop all active voting channels before tallying")
                        }
                    }
                },
            }
        };
        if let Some(reason) = reason {
            return Err(TallyValidationError::new(format!(
                "Election {id}: {reason}."
            )));
        }
    }
    Ok(())
}

/// A Post gets one initialization report: once it is generated
/// (`initialization_report_generated`), another one is refused. A report of
/// some of its countries (`per_country`, initialization scope Post and
/// country) is not: a country published after the Post was initialized
/// still needs its own.
pub fn validate_initialization_not_generated(
    elections: &[Election],
    selected_ids: &[String],
    tally_type: &TallyType,
    per_country: bool,
) -> Result<(), TallyValidationError> {
    if *tally_type != TallyType::INITIALIZATION_REPORT || per_country {
        return Ok(());
    }
    let generated = selected_ids.iter().find(|id| {
        elections.iter().any(|election| {
            election.id == **id && election.initialization_report_generated == Some(true)
        })
    });
    match generated {
        Some(id) => Err(TallyValidationError::new(format!(
            "Election {id}: its initialization report was already generated."
        ))),
        None => Ok(()),
    }
}

/// A second initialization report is refused while another non-cancelled
/// initialization-report session (`blocking`) overlaps this one, because the
/// generated flag is only written when that session finishes. A session
/// without a country filter covers the whole Post and overlaps every other;
/// two country-filtered sessions overlap when they share a country.
pub fn validate_no_concurrent_initialization(
    tally_type: &TallyType,
    blocking: &[TallySession],
    area_filter: Option<&[String]>,
) -> Result<(), TallyValidationError> {
    if *tally_type != TallyType::INITIALIZATION_REPORT {
        return Ok(());
    }
    let overlapping =
        blocking.iter().find(
            |session| match (area_filter, initialization_area_filter(session)) {
                (Some(areas), Some(other)) => other.iter().any(|area| areas.contains(area)),
                _ => true,
            },
        );
    match overlapping {
        Some(session) => Err(TallyValidationError::new(format!(
            "Another initialization report tally ({}) already exists for these elections.",
            session.id
        ))),
        None => Ok(()),
    }
}

/// A ballot box the tally counts: one area of one election, with the names
/// a refusal shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedBallotBox {
    pub election_id: String,
    pub election_name: String,
    pub area_id: String,
    pub area_name: String,
}

/// Why a ballot box is not ready to tally, as the refusal shows it.
fn not_ready_reason(status: Option<BallotBoxSealStatus>) -> Option<&'static str> {
    match status {
        Some(BallotBoxSealStatus::Published) => None,
        None | Some(BallotBoxSealStatus::Pending) => Some("not sealed yet"),
        Some(BallotBoxSealStatus::Sealed) => Some("sealed, not yet on the bulletin board"),
        Some(BallotBoxSealStatus::Failed) => Some("not sealed: incident"),
    }
}

/// With the event sealing ballot boxes at close, electoral results are
/// tallied only once every ballot box they count has its seal on the
/// bulletin board (`published`). Initialization reports count no ballots
/// and are never refused here.
pub fn validate_ballot_boxes_sealed(
    policy: BallotBoxSealPolicy,
    tally_type: &TallyType,
    expected_boxes: &[ExpectedBallotBox],
    seals: &[BallotBoxSealState],
) -> Result<(), TallyValidationError> {
    if policy != BallotBoxSealPolicy::SEAL_AT_CLOSE
        || *tally_type == TallyType::INITIALIZATION_REPORT
    {
        return Ok(());
    }
    // Election name, then area name, so the refusal reads the same each time.
    let mut not_ready: BTreeMap<(&str, &str), Vec<(&str, &str, &str)>> = BTreeMap::new();
    for expected in expected_boxes {
        let status = seals
            .iter()
            .find(|seal| {
                seal.election_id == expected.election_id && seal.area_id == expected.area_id
            })
            .map(|seal| seal.status);
        if let Some(reason) = not_ready_reason(status) {
            not_ready
                .entry((
                    expected.election_name.as_str(),
                    expected.election_id.as_str(),
                ))
                .or_default()
                .push((
                    expected.area_name.as_str(),
                    expected.area_id.as_str(),
                    reason,
                ));
        }
    }
    if not_ready.is_empty() {
        return Ok(());
    }
    let elections: Vec<String> = not_ready
        .into_iter()
        .map(|((election_name, _), mut areas)| {
            areas.sort();
            let areas: Vec<String> = areas
                .into_iter()
                .map(|(area_name, _, reason)| format!("{area_name} ({reason})"))
                .collect();
            format!("{election_name}: {}", areas.join(", "))
        })
        .collect();
    Err(TallyValidationError::new(format!(
        "Every ballot box must be sealed and its seal on the bulletin board before \
         tallying. Not ready: {}.",
        elections.join("; ")
    )))
}

/// Refuses a tally that leaves out a sealed ballot box with ballots: every
/// such box must have the tally's contests in its area (`tallied`), or its
/// ballots would silently be missing from the results. `boxes` names the
/// boxes.
pub fn validate_sealed_boxes_tallied(
    boxes: &[ExpectedBallotBox],
    seals: &[BallotBoxSealState],
    tallied: &BTreeSet<(String, String)>,
) -> Result<(), TallyValidationError> {
    let mut missing: Vec<(&str, &str)> = seals
        .iter()
        .filter(|seal| seal.ballots_in_box.unwrap_or_default() > 0)
        .filter(|seal| !tallied.contains(&(seal.election_id.clone(), seal.area_id.clone())))
        .map(|seal| {
            boxes
                .iter()
                .find(|named| {
                    named.election_id == seal.election_id && named.area_id == seal.area_id
                })
                .map(|named| (named.election_name.as_str(), named.area_name.as_str()))
                .unwrap_or((seal.election_id.as_str(), seal.area_id.as_str()))
        })
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    missing.sort();
    let names: Vec<String> = missing
        .iter()
        .map(|(election, area)| format!("{election}, {area}"))
        .collect();
    Err(TallyValidationError::new(if names.len() == 1 {
        format!(
            "The sealed ballot box of {} is not in this tally: its area no longer has the \
             election's contests.",
            names[0]
        )
    } else {
        format!(
            "The sealed ballot boxes of {} are not in this tally: their areas no longer have \
             the election's contests.",
            names.join("; ")
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn initialization_session(id: &str, areas: Option<&[&str]>) -> TallySession {
        let annotations = areas.map(|areas| json!({"initialization_area_ids": areas}));
        serde_json::from_value(json!({
            "id": id, "tenant_id": "tenant", "election_event_id": "event",
            "keys_ceremony_id": "keys", "threshold": 2, "is_execution_completed": false,
            "tally_type": "INITIALIZATION_REPORT", "annotations": annotations,
        }))
        .expect("a tally session")
    }

    fn areas(areas: &[&str]) -> Vec<String> {
        areas.iter().map(|area| area.to_string()).collect()
    }

    #[test]
    fn concurrent_initialization_is_refused_unless_the_countries_are_disjoint() {
        let whole_post = initialization_session("whole", None);
        let north = initialization_session("north", Some(&["north"]));
        let report = TallyType::INITIALIZATION_REPORT;
        assert!(validate_no_concurrent_initialization(&report, &[], None).is_ok());
        let refused = |blocking: &[TallySession], filter: Option<&[String]>| {
            validate_no_concurrent_initialization(&report, blocking, filter)
                .expect_err("refused")
                .to_string()
        };
        assert_eq!(
            refused(&[whole_post.clone()], None),
            "Another initialization report tally (whole) already exists for these elections."
        );
        assert!(refused(&[north.clone()], None).contains("(north)"));
        assert!(refused(&[whole_post], Some(&areas(&["north"]))).contains("(whole)"));
        assert!(refused(&[north.clone()], Some(&areas(&["north", "south"]))).contains("(north)"));
        assert!(
            validate_no_concurrent_initialization(&report, &[north], Some(&areas(&["south"])))
                .is_ok()
        );
    }

    #[test]
    fn electoral_results_ignore_concurrent_initialization_sessions() {
        let whole_post = initialization_session("whole", None);
        assert!(validate_no_concurrent_initialization(
            &TallyType::ELECTORAL_RESULTS,
            &[whole_post],
            None
        )
        .is_ok());
    }

    fn election(id: &str, voting_status: &str) -> Election {
        serde_json::from_value(json!({
            "id": id, "tenant_id": "tenant", "election_event_id": "open-event",
            "status": {"is_published": true, "voting_status": voting_status,
                       "allow_tally": "requires-voting-period-end"}
        }))
        .unwrap()
    }

    #[test]
    fn closed_election_can_be_tallied_while_another_election_is_open() {
        let elections = vec![election("closed", "CLOSED"), election("open", "OPEN")];
        assert!(validate_tally_elections(
            &elections,
            &["closed".into()],
            TallyType::ELECTORAL_RESULTS
        )
        .is_ok());
        assert!(validate_tally_elections(
            &elections,
            &["closed".into(), "open".into()],
            TallyType::ELECTORAL_RESULTS
        )
        .is_err());
    }

    #[test]
    fn policy_checks_every_active_channel_including_telephone() {
        for channel in [
            "voting_status",
            "kiosk_voting_status",
            "early_voting_status",
            "telephone_voting_status",
        ] {
            for status in ["OPEN", "PAUSED"] {
                let mut selected = election("selected", "CLOSED");
                selected.status.as_mut().unwrap()[channel] = json!(status);
                let error = validate_tally_elections(
                    &[selected],
                    &["selected".into()],
                    TallyType::ELECTORAL_RESULTS,
                )
                .unwrap_err();
                assert!(error
                    .to_string()
                    .contains("Election selected: end its voting period"));
            }
        }
    }

    #[test]
    fn secondary_channels_only_block_tally_when_enabled() {
        for (channel, status_field) in [
            ("kiosk", "kiosk_voting_status"),
            ("early_voting", "early_voting_status"),
            ("telephone", "telephone_voting_status"),
        ] {
            for status in ["OPEN", "PAUSED"] {
                for enabled in [Some(true), Some(false), None] {
                    let mut selected = election("selected", "CLOSED");
                    selected.status.as_mut().unwrap()[status_field] = json!(status);
                    selected.voting_channels = Some(json!({channel: enabled}));
                    assert_eq!(
                        validate_tally_elections(
                            &[selected.clone()],
                            &["selected".into()],
                            TallyType::ELECTORAL_RESULTS,
                        )
                        .is_ok(),
                        enabled != Some(true),
                        "{channel}: {status}, enabled={enabled:?}"
                    );
                    selected.voting_channels = Some(json!({}));
                    assert!(validate_tally_elections(
                        &[selected],
                        &["selected".into()],
                        TallyType::ELECTORAL_RESULTS,
                    )
                    .is_ok());
                }
            }
        }
    }

    #[test]
    fn disabled_secondary_channels_do_not_bypass_online_status() {
        let mut selected = election("selected", "OPEN");
        selected.voting_channels = Some(json!({}));
        assert!(validate_tally_elections(
            &[selected],
            &["selected".into()],
            TallyType::ELECTORAL_RESULTS,
        )
        .is_err());
    }

    #[test]
    fn malformed_channel_configuration_returns_election_context() {
        let mut selected = election("selected", "CLOSED");
        selected.voting_channels = Some(json!({"telephone": "true"}));
        let error = validate_tally_elections(
            &[selected],
            &["selected".into()],
            TallyType::ELECTORAL_RESULTS,
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("Election selected has invalid voting_channels"));
    }

    #[test]
    fn explicit_policy_and_initialization_reports_keep_their_own_rules() {
        let mut selected = election("selected", "OPEN");
        selected.status.as_mut().unwrap()["allow_tally"] = json!("allowed");
        assert!(validate_tally_elections(
            &[selected.clone()],
            &["selected".into()],
            TallyType::ELECTORAL_RESULTS
        )
        .is_ok());
        selected.status.as_mut().unwrap()["allow_tally"] = json!("disallowed");
        assert!(validate_tally_elections(
            &[selected.clone()],
            &["selected".into()],
            TallyType::ELECTORAL_RESULTS
        )
        .is_err());
        selected.status.as_mut().unwrap()["voting_status"] = json!("NOT_STARTED");
        assert!(validate_tally_elections(
            &[selected.clone()],
            &["selected".into()],
            TallyType::INITIALIZATION_REPORT
        )
        .is_ok());
        selected.status.as_mut().unwrap()["init_report"] = json!("disallowed");
        assert!(validate_tally_elections(
            &[selected],
            &["selected".into()],
            TallyType::INITIALIZATION_REPORT
        )
        .is_err());
    }

    #[test]
    fn initialization_report_requires_every_channel_not_started() {
        for channel in [
            "voting_status",
            "kiosk_voting_status",
            "early_voting_status",
            "telephone_voting_status",
        ] {
            for status in ["OPEN", "PAUSED", "CLOSED"] {
                let mut selected = election("selected", "NOT_STARTED");
                selected.status.as_mut().unwrap()[channel] = json!(status);
                selected.voting_channels = Some(json!({}));
                let error = validate_tally_elections(
                    &[selected],
                    &["selected".into()],
                    TallyType::INITIALIZATION_REPORT,
                )
                .unwrap_err();
                assert!(
                    error.to_string().contains(
                        "Election selected: generate its initialization report before any voting channel opens"
                    ),
                    "{channel}: {status}"
                );
            }
        }
    }

    #[test]
    fn initialization_report_is_generated_only_once() {
        let mut selected = election("selected", "NOT_STARTED");
        let other = election("other", "NOT_STARTED");
        let ids = ["other".to_string(), "selected".to_string()];
        let check = |selected: &Election, tally_type: TallyType, per_country: bool| {
            validate_initialization_not_generated(
                &[other.clone(), selected.clone()],
                &ids,
                &tally_type,
                per_country,
            )
        };
        for generated in [None, Some(false)] {
            selected.initialization_report_generated = generated;
            assert!(check(&selected, TallyType::INITIALIZATION_REPORT, false).is_ok());
        }
        selected.initialization_report_generated = Some(true);
        let error = check(&selected, TallyType::INITIALIZATION_REPORT, false)
            .expect_err("a second report of the Post is refused");
        assert_eq!(
            error.to_string(),
            "Election selected: its initialization report was already generated."
        );
        // A country published after the Post was initialized gets its own
        // report, and electoral results aren't initialization reports.
        assert!(check(&selected, TallyType::INITIALIZATION_REPORT, true).is_ok());
        assert!(check(&selected, TallyType::ELECTORAL_RESULTS, false).is_ok());
    }

    #[test]
    fn invalid_selection_or_status_never_bypasses_validation() {
        let selected = election("selected", "CLOSED");
        for ids in [
            vec![],
            vec!["missing".into()],
            vec!["selected".into(), "selected".into()],
        ] {
            assert!(validate_tally_elections(
                &[selected.clone()],
                &ids,
                TallyType::ELECTORAL_RESULTS
            )
            .is_err());
        }
        for status in [
            None,
            Some(json!({"voting_status": "INVALID"})),
            Some(json!({"is_published": false})),
        ] {
            let mut selected = selected.clone();
            selected.status = status;
            assert!(validate_tally_elections(
                &[selected],
                &["selected".into()],
                TallyType::ELECTORAL_RESULTS
            )
            .is_err());
        }
    }

    fn expected(area: &str) -> ExpectedBallotBox {
        ExpectedBallotBox {
            election_id: "e1".into(),
            election_name: "Mayor".into(),
            area_id: area.into(),
            area_name: format!("Area {area}"),
        }
    }

    fn seal(area: &str, status: BallotBoxSealStatus) -> BallotBoxSealState {
        BallotBoxSealState {
            election_id: "e1".into(),
            area_id: area.into(),
            status,
            ballots_in_box: Some(1),
        }
    }

    fn sealed(
        policy: BallotBoxSealPolicy,
        tally_type: TallyType,
        seals: &[BallotBoxSealState],
    ) -> Result<(), TallyValidationError> {
        validate_ballot_boxes_sealed(
            policy,
            &tally_type,
            &[expected("north"), expected("south")],
            seals,
        )
    }

    #[test]
    fn electoral_results_wait_for_every_seal_on_the_bulletin_board() {
        use BallotBoxSealStatus::*;
        let on = BallotBoxSealPolicy::SEAL_AT_CLOSE;
        assert!(sealed(
            on,
            TallyType::ELECTORAL_RESULTS,
            &[seal("north", Published), seal("south", Published)]
        )
        .is_ok());
        for (south, reason) in [
            (None, "not sealed yet"),
            (Some(Pending), "not sealed yet"),
            (Some(Sealed), "sealed, not yet on the bulletin board"),
            (Some(Failed), "not sealed: incident"),
        ] {
            let mut seals = vec![seal("north", Published)];
            seals.extend(south.map(|status| seal("south", status)));
            let error = sealed(on, TallyType::ELECTORAL_RESULTS, &seals).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!(
                    "Every ballot box must be sealed and its seal on the bulletin board \
                     before tallying. Not ready: Mayor: Area south ({reason})."
                )
            );
        }
    }

    #[test]
    fn the_refusal_names_every_box_that_is_not_ready_in_order() {
        let error = validate_ballot_boxes_sealed(
            BallotBoxSealPolicy::SEAL_AT_CLOSE,
            &TallyType::ELECTORAL_RESULTS,
            &[
                expected("south"),
                ExpectedBallotBox {
                    election_id: "e0".into(),
                    election_name: "Council".into(),
                    area_id: "west".into(),
                    area_name: "Area west".into(),
                },
                expected("north"),
            ],
            &[seal("north", BallotBoxSealStatus::Failed)],
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Every ballot box must be sealed and its seal on the bulletin board before \
             tallying. Not ready: Council: Area west (not sealed yet); Mayor: Area north \
             (not sealed: incident), Area south (not sealed yet)."
        );
    }

    #[test]
    fn initialization_reports_and_events_that_do_not_seal_skip_the_seal_check() {
        assert!(sealed(
            BallotBoxSealPolicy::SEAL_AT_CLOSE,
            TallyType::INITIALIZATION_REPORT,
            &[]
        )
        .is_ok());
        assert!(sealed(
            BallotBoxSealPolicy::DO_NOT_SEAL,
            TallyType::ELECTORAL_RESULTS,
            &[]
        )
        .is_ok());
    }

    #[test]
    fn a_sealed_box_with_ballots_must_be_in_the_tally() {
        let tallied = BTreeSet::from([("e1".to_string(), "north".to_string())]);
        let boxes = [expected("north"), expected("south"), expected("east")];
        let mut empty = seal("east", BallotBoxSealStatus::Published);
        empty.ballots_in_box = Some(0);
        // The north box is tallied and the east box has no ballots.
        assert!(validate_sealed_boxes_tallied(
            &boxes,
            &[seal("north", BallotBoxSealStatus::Published), empty.clone()],
            &tallied
        )
        .is_ok());
        let error = validate_sealed_boxes_tallied(
            &boxes,
            &[
                seal("north", BallotBoxSealStatus::Published),
                seal("south", BallotBoxSealStatus::Published),
                empty,
            ],
            &tallied,
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "The sealed ballot box of Mayor, Area south is not in this tally: its area no \
             longer has the election's contests."
        );
        let mut unnamed = seal("west", BallotBoxSealStatus::Published);
        unnamed.election_id = "e2".into();
        let error = validate_sealed_boxes_tallied(
            &boxes,
            &[seal("south", BallotBoxSealStatus::Published), unnamed],
            &tallied,
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "The sealed ballot boxes of Mayor, Area south; e2, west are not in this tally: \
             their areas no longer have the election's contests."
        );
    }
}
