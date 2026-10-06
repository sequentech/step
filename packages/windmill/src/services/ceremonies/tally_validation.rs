// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::ballot::{AllowTallyStatus, ElectionStatus, InitReport};
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::{Election, VotingChannels};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct TallyValidationError(String);

impl TallyValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

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
}
