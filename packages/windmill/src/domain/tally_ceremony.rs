// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Rules of the tally ceremony: which status changes are allowed, when enough
//! trustees have restored their keys, and when a session can be recounted.

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

/// The trustee named `trustee_name`, who must still be waiting to restore
/// their key.
pub fn waiting_trustee<'a>(
    status: &'a TallyCeremonyStatus,
    trustee_name: &str,
) -> Result<&'a TallyTrustee> {
    let trustee = status
        .trustees
        .iter()
        .find(|trustee| trustee.name == trustee_name)
        .ok_or_else(|| anyhow!("Trustee not part of the keys ceremony or has invalid state"))?;
    if trustee.status != TallyTrusteeStatus::WAITING {
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

/// Whether enough trustees restored their key for the session to connect.
pub fn reaches_key_threshold(status: &TallyCeremonyStatus, threshold: i64) -> bool {
    restored_trustee_count(status) as i64 >= threshold
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
