// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Rules of the tally ceremony: which status changes are allowed, when enough
//! trustees have restored their keys, and when a session can be recounted.

use crate::domain::trustee_signatures::TrusteeSignatures;
use anyhow::{anyhow, Result};
use sequent_core::types::ceremonies::{
    TallyCeremonyStatus, TallyElection, TallyElectionStatus, TallyExecutionStatus, TallyTrustee,
    TallyTrusteeStatus,
};
use sequent_core::types::hasura::core::TallySession;
use serde_json::Value;
use std::str::FromStr;
use thiserror::Error;

/// Tally session annotation naming the user who created the session.
pub const EXECUTER_USERNAME_ANNOTATION: &str = "executer_username";
/// Tally session annotation holding the id of the user who created the session.
pub const EXECUTER_USER_ID_ANNOTATION: &str = "executer_user_id";

/// A tally request that breaks a rule. Harvest answers it with a 400 and shows
/// the message to the user.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct TallyValidationError(String);

impl TallyValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

/// Who created a tally session, as recorded in its annotations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TallyExecuter {
    pub user_id: Option<String>,
    pub username: Option<String>,
}

/// A missing or unknown execution status reads as `STARTED`.
pub fn tally_execution_status(execution_status: Option<&str>) -> TallyExecutionStatus {
    execution_status
        .and_then(|value| TallyExecutionStatus::from_str(value).ok())
        .unwrap_or(TallyExecutionStatus::STARTED)
}

/// The statuses an administrator can move a tally session to. `SUCCESS` and
/// `CANCELLED` are final.
pub fn allowed_status_changes(current: &TallyExecutionStatus) -> &'static [TallyExecutionStatus] {
    match current {
        TallyExecutionStatus::STARTED | TallyExecutionStatus::IN_PROGRESS => {
            &[TallyExecutionStatus::CANCELLED]
        }
        TallyExecutionStatus::CONNECTED | TallyExecutionStatus::AWAITING_INPUT => &[
            TallyExecutionStatus::IN_PROGRESS,
            TallyExecutionStatus::CANCELLED,
        ],
        TallyExecutionStatus::SUCCESS | TallyExecutionStatus::CANCELLED => &[],
    }
}

pub fn check_status_change(
    current: &TallyExecutionStatus,
    new: &TallyExecutionStatus,
) -> Result<(), TallyValidationError> {
    if allowed_status_changes(current).contains(new) {
        Ok(())
    } else {
        Err(TallyValidationError::new(format!(
            "Cannot change tally status from {current} to {new}."
        )))
    }
}

pub fn restored_trustee_count(status: &TallyCeremonyStatus) -> usize {
    status
        .trustees
        .iter()
        .filter(|trustee| trustee.status == TallyTrusteeStatus::KEY_RESTORED)
        .count()
}

/// Until `threshold` trustees have restored their key, the session can only
/// be cancelled.
pub fn check_trustee_quorum(
    threshold: i64,
    restored_trustees: usize,
    new: &TallyExecutionStatus,
) -> Result<(), TallyValidationError> {
    if threshold > restored_trustees as i64 && *new != TallyExecutionStatus::CANCELLED {
        return Err(TallyValidationError::new(format!(
            "Insufficient number of connected trustees {restored_trustees}. Required threshold {threshold}."
        )));
    }
    Ok(())
}

/// Keys can only be restored before the tally starts.
pub fn check_key_restore_status(current: &TallyExecutionStatus) -> Result<()> {
    if *current != TallyExecutionStatus::STARTED && *current != TallyExecutionStatus::CONNECTED {
        return Err(anyhow!("Unexpected status {current}"));
    }
    Ok(())
}

/// The trustee named `trustee_name` who may restore their key now: one
/// still waiting, or, when the rule makes trustees sign, one who restored
/// it unsigned before.
pub fn restoring_trustee<'a>(
    status: &'a TallyCeremonyStatus,
    trustee_name: &str,
    signatures: &TrusteeSignatures,
) -> Result<&'a TallyTrustee> {
    let trustee = status
        .trustees
        .iter()
        .find(|trustee| trustee.name == trustee_name)
        .ok_or_else(|| anyhow!("Trustee not part of the keys ceremony or has invalid state"))?;
    let redo =
        trustee.status == TallyTrusteeStatus::KEY_RESTORED && signatures.may_redo(trustee_name);
    if trustee.status != TallyTrusteeStatus::WAITING && !redo {
        return Err(anyhow!("Unexpected trustee status {}", trustee.status));
    }
    Ok(trustee)
}

/// `status` with `trustee_name` marked as having restored their key.
pub fn restore_trustee_key(
    mut status: TallyCeremonyStatus,
    trustee_name: &str,
) -> TallyCeremonyStatus {
    for trustee in status
        .trustees
        .iter_mut()
        .filter(|trustee| trustee.name == trustee_name)
    {
        trustee.status = TallyTrusteeStatus::KEY_RESTORED;
    }
    status
}

/// Whether enough trustees restored their key for the session to connect;
/// when the rule makes trustees sign, only signed restores count.
pub fn reaches_key_threshold(
    status: &TallyCeremonyStatus,
    threshold: i64,
    signatures: &TrusteeSignatures,
) -> bool {
    status
        .trustees
        .iter()
        .filter(|trustee| {
            trustee.status == TallyTrusteeStatus::KEY_RESTORED && signatures.counts(&trustee.name)
        })
        .count() as i64
        >= threshold
}

/// Only a session that completed successfully can be recounted.
pub fn is_recount_eligible(tally_session: &TallySession) -> bool {
    tally_session.execution_status.as_deref()
        == Some(TallyExecutionStatus::SUCCESS.to_string().as_str())
        && tally_session.is_execution_completed
}

/// A recount tallies every election again from the start.
pub fn recount_elections_status(election_ids: &[String]) -> Vec<TallyElection> {
    election_ids
        .iter()
        .map(|election_id| TallyElection {
            election_id: election_id.clone(),
            status: TallyElectionStatus::WAITING,
            progress: 0.0,
        })
        .collect()
}

/// Values that are missing or not strings are ignored.
pub fn tally_executer(annotations: Option<&Value>) -> TallyExecuter {
    let read = |key: &str| {
        annotations
            .and_then(|annotations| annotations.get(key))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    TallyExecuter {
        user_id: read(EXECUTER_USER_ID_ANNOTATION),
        username: read(EXECUTER_USERNAME_ANNOTATION),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::ceremonies::TallyExecutionStatus::{
        AWAITING_INPUT, CANCELLED, CONNECTED, IN_PROGRESS, STARTED, SUCCESS,
    };
    use serde_json::json;

    const ALL_STATUSES: [TallyExecutionStatus; 6] = [
        STARTED,
        CONNECTED,
        IN_PROGRESS,
        AWAITING_INPUT,
        SUCCESS,
        CANCELLED,
    ];

    fn ceremony_status(trustees: &[(&str, TallyTrusteeStatus)]) -> TallyCeremonyStatus {
        TallyCeremonyStatus {
            trustees: trustees
                .iter()
                .map(|(name, status)| TallyTrustee {
                    name: name.to_string(),
                    status: status.clone(),
                })
                .collect(),
            ..Default::default()
        }
    }

    fn session(execution_status: Option<&str>, is_execution_completed: bool) -> TallySession {
        serde_json::from_value(json!({
            "id": "session", "tenant_id": "tenant", "election_event_id": "event",
            "keys_ceremony_id": "keys", "threshold": 2,
            "execution_status": execution_status,
            "is_execution_completed": is_execution_completed,
        }))
        .unwrap()
    }

    #[test]
    fn missing_or_unknown_execution_status_reads_as_started() {
        assert_eq!(tally_execution_status(None), STARTED);
        assert_eq!(tally_execution_status(Some("PAUSED")), STARTED);
        assert_eq!(tally_execution_status(Some("in_progress")), STARTED);
        for status in ALL_STATUSES {
            assert_eq!(tally_execution_status(Some(&status.to_string())), status);
        }
    }

    #[test]
    fn only_the_documented_status_changes_are_allowed() {
        let allowed = [
            (STARTED, CANCELLED),
            (CONNECTED, IN_PROGRESS),
            (CONNECTED, CANCELLED),
            (IN_PROGRESS, CANCELLED),
            (AWAITING_INPUT, IN_PROGRESS),
            (AWAITING_INPUT, CANCELLED),
        ];
        for current in ALL_STATUSES {
            for new in ALL_STATUSES {
                let result = check_status_change(&current, &new);
                if allowed.contains(&(current.clone(), new.clone())) {
                    assert!(result.is_ok(), "{current} -> {new} should be allowed");
                } else {
                    assert_eq!(
                        result.unwrap_err().to_string(),
                        format!("Cannot change tally status from {current} to {new}."),
                    );
                }
            }
        }
    }

    #[test]
    fn finished_sessions_cannot_change_status() {
        assert!(allowed_status_changes(&SUCCESS).is_empty());
        assert!(allowed_status_changes(&CANCELLED).is_empty());
    }

    #[test]
    fn restored_trustees_are_counted_by_status() {
        let status = ceremony_status(&[
            ("alice", TallyTrusteeStatus::KEY_RESTORED),
            ("bob", TallyTrusteeStatus::WAITING),
            ("carol", TallyTrusteeStatus::KEY_RESTORED),
        ]);
        assert_eq!(restored_trustee_count(&status), 2);
    }

    #[test]
    fn fewer_restored_trustees_than_the_threshold_only_allow_cancelling() {
        for new in [STARTED, CONNECTED, IN_PROGRESS, AWAITING_INPUT, SUCCESS] {
            assert_eq!(
                check_trustee_quorum(3, 2, &new).unwrap_err().to_string(),
                "Insufficient number of connected trustees 2. Required threshold 3."
            );
        }
        assert!(check_trustee_quorum(3, 2, &CANCELLED).is_ok());
        assert!(check_trustee_quorum(3, 0, &CANCELLED).is_ok());
    }

    #[test]
    fn reaching_the_threshold_satisfies_the_quorum() {
        assert!(check_trustee_quorum(3, 3, &IN_PROGRESS).is_ok());
        assert!(check_trustee_quorum(3, 4, &IN_PROGRESS).is_ok());
        assert!(check_trustee_quorum(0, 0, &IN_PROGRESS).is_ok());
    }

    #[test]
    fn keys_can_only_be_restored_while_started_or_connected() {
        assert!(check_key_restore_status(&STARTED).is_ok());
        assert!(check_key_restore_status(&CONNECTED).is_ok());
        for status in [IN_PROGRESS, AWAITING_INPUT, SUCCESS, CANCELLED] {
            assert_eq!(
                check_key_restore_status(&status).unwrap_err().to_string(),
                format!("Unexpected status {status}")
            );
        }
    }

    #[test]
    fn a_waiting_trustee_can_restore_their_key() {
        let status = ceremony_status(&[
            ("alice", TallyTrusteeStatus::KEY_RESTORED),
            ("bob", TallyTrusteeStatus::WAITING),
        ]);
        assert_eq!(
            restoring_trustee(&status, "bob", &TrusteeSignatures::NotNeeded)
                .unwrap()
                .name,
            "bob"
        );
    }

    #[test]
    fn a_trustee_outside_the_ceremony_cannot_restore_a_key() {
        let status = ceremony_status(&[("alice", TallyTrusteeStatus::WAITING)]);
        assert_eq!(
            restoring_trustee(&status, "mallory", &TrusteeSignatures::NotNeeded)
                .unwrap_err()
                .to_string(),
            "Trustee not part of the keys ceremony or has invalid state"
        );
    }

    #[test]
    fn a_trustee_cannot_restore_their_key_twice() {
        let status = ceremony_status(&[("alice", TallyTrusteeStatus::KEY_RESTORED)]);
        assert_eq!(
            restoring_trustee(&status, "alice", &TrusteeSignatures::NotNeeded)
                .unwrap_err()
                .to_string(),
            "Unexpected trustee status KEY_RESTORED"
        );
    }

    #[test]
    fn restoring_a_key_changes_only_that_trustee() {
        let status = ceremony_status(&[
            ("alice", TallyTrusteeStatus::WAITING),
            ("bob", TallyTrusteeStatus::WAITING),
        ]);
        let restored = restore_trustee_key(status, "bob");
        assert_eq!(
            restored
                .trustees
                .iter()
                .map(|trustee| (trustee.name.as_str(), trustee.status.clone()))
                .collect::<Vec<_>>(),
            vec![
                ("alice", TallyTrusteeStatus::WAITING),
                ("bob", TallyTrusteeStatus::KEY_RESTORED),
            ]
        );
    }

    #[test]
    fn the_key_threshold_is_reached_at_exactly_the_threshold() {
        let one = ceremony_status(&[
            ("alice", TallyTrusteeStatus::KEY_RESTORED),
            ("bob", TallyTrusteeStatus::WAITING),
        ]);
        let two = ceremony_status(&[
            ("alice", TallyTrusteeStatus::KEY_RESTORED),
            ("bob", TallyTrusteeStatus::KEY_RESTORED),
        ]);
        let any = TrusteeSignatures::NotNeeded;
        assert!(!reaches_key_threshold(&one, 2, &any));
        assert!(reaches_key_threshold(&two, 2, &any));
        assert!(reaches_key_threshold(&two, 1, &any));
    }

    #[test]
    fn with_signatures_needed_only_signed_restores_reach_the_threshold() {
        let two = ceremony_status(&[
            ("alice", TallyTrusteeStatus::KEY_RESTORED),
            ("bob", TallyTrusteeStatus::KEY_RESTORED),
        ]);
        let only_bob = TrusteeSignatures::Needed {
            signed: Default::default(),
            signing: Some("bob".into()),
        };
        assert!(!reaches_key_threshold(&two, 2, &only_bob));
        assert!(reaches_key_threshold(&two, 1, &only_bob));
    }

    #[test]
    fn with_signatures_needed_an_unsigned_restore_is_redone_signed_once() {
        let status = ceremony_status(&[
            ("alice", TallyTrusteeStatus::KEY_RESTORED),
            ("bob", TallyTrusteeStatus::WAITING),
        ]);
        let none_signed = TrusteeSignatures::Needed {
            signed: Default::default(),
            signing: Some("alice".into()),
        };
        assert_eq!(
            restoring_trustee(&status, "alice", &none_signed)
                .unwrap()
                .name,
            "alice"
        );
        let alice_signed = TrusteeSignatures::Needed {
            signed: ["alice".to_string()].into(),
            signing: None,
        };
        assert_eq!(
            restoring_trustee(&status, "alice", &alice_signed)
                .unwrap_err()
                .to_string(),
            "Unexpected trustee status KEY_RESTORED"
        );
        assert_eq!(
            restoring_trustee(&status, "alice", &TrusteeSignatures::NotNeeded)
                .unwrap_err()
                .to_string(),
            "Unexpected trustee status KEY_RESTORED"
        );
        assert_eq!(
            restoring_trustee(&status, "bob", &TrusteeSignatures::NotNeeded)
                .unwrap()
                .name,
            "bob"
        );
    }

    #[test]
    fn only_successfully_completed_sessions_are_recount_eligible() {
        assert!(is_recount_eligible(&session(Some("SUCCESS"), true)));
        assert!(!is_recount_eligible(&session(Some("SUCCESS"), false)));
        assert!(!is_recount_eligible(&session(Some("CANCELLED"), true)));
        assert!(!is_recount_eligible(&session(Some("success"), true)));
        assert!(!is_recount_eligible(&session(None, true)));
    }

    #[test]
    fn a_recount_restarts_every_election_from_zero() {
        let elections = recount_elections_status(&["first".to_string(), "second".to_string()]);
        assert_eq!(
            elections
                .iter()
                .map(|election| (
                    election.election_id.as_str(),
                    election.status.clone(),
                    election.progress
                ))
                .collect::<Vec<_>>(),
            vec![
                ("first", TallyElectionStatus::WAITING, 0.0),
                ("second", TallyElectionStatus::WAITING, 0.0),
            ]
        );
    }

    #[test]
    fn the_executer_is_read_from_the_annotations() {
        let annotations = json!({"executer_user_id": "user-1", "executer_username": "admin"});
        assert_eq!(
            tally_executer(Some(&annotations)),
            TallyExecuter {
                user_id: Some("user-1".to_string()),
                username: Some("admin".to_string()),
            }
        );
    }

    #[test]
    fn missing_or_malformed_executer_annotations_are_ignored() {
        assert_eq!(tally_executer(None), TallyExecuter::default());
        assert_eq!(tally_executer(Some(&json!([]))), TallyExecuter::default());
        let annotations = json!({"executer_user_id": 7, "executer_username": null});
        assert_eq!(tally_executer(Some(&annotations)), TallyExecuter::default());
        let partial = json!({"executer_username": "admin"});
        assert_eq!(
            tally_executer(Some(&partial)),
            TallyExecuter {
                user_id: None,
                username: Some("admin".to_string()),
            }
        );
    }
}
