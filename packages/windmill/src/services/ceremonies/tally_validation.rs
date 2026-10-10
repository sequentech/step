// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

pub use crate::domain::tally_ceremony::TallyValidationError;
use crate::ports::tally_ceremony::BallotBoxSealState;
use crate::postgres::ballot_box_seal::BallotBoxSealStatus;
use sequent_core::ballot::{AllowTallyStatus, BallotBoxSealPolicy, ElectionStatus, InitReport};
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::{Election, VotingChannels};
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
