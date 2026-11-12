// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::ballot::{ElectionStatus, InitReport, VotingStatus};
use sequent_core::types::hasura::core::Election;
use thiserror::Error;

/// A tally request refused because of the state of its elections. Harvest
/// answers it with 400 Bad Request.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct TallyValidationError(String);

impl TallyValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

/// An initialization report is a test tally of the still empty ballot
/// boxes. Each selected election must allow it, must not have generated its
/// report yet, and must have every voting channel NOT_STARTED: otherwise the
/// report would decrypt real cast ballots.
pub fn validate_initialization_report(
    elections: &[Election],
    selected_ids: &[String],
) -> Result<(), TallyValidationError> {
    for id in selected_ids {
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
        let reason = if status.init_report != InitReport::ALLOWED {
            Some("initialization reports are not allowed")
        } else if election.initialization_report_generated == Some(true) {
            Some("its initialization report was already generated")
        } else if status.voting_status != VotingStatus::NOT_STARTED
            || status.kiosk_voting_status != VotingStatus::NOT_STARTED
        {
            Some("generate its initialization report before any voting channel opens")
        } else {
            None
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

    fn election(id: &str) -> Election {
        serde_json::from_value(json!({
            "id": id, "tenant_id": "tenant", "election_event_id": "event",
            "name": id,
            "status": {"is_published": true, "voting_status": "NOT_STARTED",
                       "kiosk_voting_status": "NOT_STARTED",
                       "init_report": "allowed"}
        }))
        .expect("election fixture")
    }

    fn error_message(elections: &[Election], selected: &[&str]) -> String {
        let selected: Vec<String> = selected.iter().map(|id| id.to_string()).collect();
        validate_initialization_report(elections, &selected)
            .expect_err("initialization report should be refused")
            .to_string()
    }

    #[test]
    fn initialization_report_is_allowed_before_any_channel_opens() {
        let mut selected = election("selected");
        for generated in [None, Some(false)] {
            selected.initialization_report_generated = generated;
            assert!(
                validate_initialization_report(&[selected.clone()], &["selected".into()]).is_ok()
            );
        }
    }

    #[test]
    fn initialization_report_requires_every_channel_not_started() {
        for channel in ["voting_status", "kiosk_voting_status"] {
            for status in ["OPEN", "PAUSED", "CLOSED"] {
                let mut selected = election("selected");
                selected.status.as_mut().expect("status")[channel] = json!(status);
                assert_eq!(
                    error_message(&[selected], &["selected"]),
                    "Election selected: generate its initialization report \
                     before any voting channel opens.",
                    "{channel}: {status}"
                );
            }
        }
    }

    #[test]
    fn initialization_report_is_generated_only_once() {
        let mut selected = election("selected");
        selected.initialization_report_generated = Some(true);
        assert_eq!(
            error_message(&[selected], &["selected"]),
            "Election selected: its initialization report was already \
             generated."
        );
    }

    #[test]
    fn initialization_report_keeps_the_init_report_policy() {
        let mut selected = election("selected");
        selected.status.as_mut().expect("status")["init_report"] = json!("disallowed");
        assert_eq!(
            error_message(&[selected], &["selected"]),
            "Election selected: initialization reports are not allowed."
        );
    }

    #[test]
    fn every_selected_election_is_checked() {
        let mut open = election("open");
        open.status.as_mut().expect("status")["voting_status"] = json!("OPEN");
        let elections = vec![election("ready"), open];
        assert!(validate_initialization_report(&elections, &["ready".into()]).is_ok());
        assert!(error_message(&elections, &["ready", "open"]).starts_with("Election open: "));
    }

    #[test]
    fn missing_election_or_status_is_refused() {
        let mut no_status = election("no-status");
        no_status.status = None;
        let mut bad_status = election("bad-status");
        bad_status.status = Some(json!({"voting_status": "UNKNOWN"}));
        let elections = vec![no_status, bad_status];
        assert_eq!(
            error_message(&elections, &["missing"]),
            "Selected election missing was not found."
        );
        assert_eq!(
            error_message(&elections, &["no-status"]),
            "Election no-status has missing or invalid status."
        );
        assert_eq!(
            error_message(&elections, &["bad-status"]),
            "Election bad-status has missing or invalid status."
        );
    }
}
