// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The administrator's operation that disables re-voting in an election
//! before voting starts. It only lowers the number of allowed votes, to one;
//! nothing here raises it.

use crate::services::election::is_election_event_locked_down_in;
use crate::services::election_event_status::lock_status_rows;
use crate::services::election_lifecycle;
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::election_lifecycle::ElectoralPhase;
use serde_json::json;

/// The `num_allowed_revotes` of an election whose voters cast one vote each.
pub const SINGLE_VOTE: i32 = 1;

/// The lifecycle notice, and electoral-log record, of the operation.
pub const REVOTING_DISABLED: &str = "RevotingDisabled";

/// Why re-voting was not disabled, in words for the administrator. Database
/// failures keep their ordinary error type.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct RevotingRefusal(String);

/// Whether voting started on a channel of the election event or of any of its
/// elections, as the database guards of the revote limit read it.
const VOTING_STARTED: &str = "SELECT sequent_backend.trusted_voting_started(e.status)
    OR EXISTS (SELECT 1 FROM sequent_backend.election x
        WHERE x.tenant_id = e.tenant_id AND x.election_event_id = e.id
          AND sequent_backend.trusted_voting_started(x.status))
    FROM sequent_backend.election_event e
    WHERE e.tenant_id = $1::text::uuid AND e.id = $2::text::uuid";

/// Whether a stored `num_allowed_revotes` lets a voter cast more than one
/// vote: zero is unlimited and a missing value counts as one vote, as
/// [`crate::services::ballot_box::get_allowed_votes`] reads it.
pub fn allows_revoting(num_allowed_revotes: Option<i32>) -> bool {
    matches!(num_allowed_revotes, Some(limit) if limit == 0 || limit > SINGLE_VOTE)
}

/// A [`RevotingRefusal`] with `message`.
fn refusal(message: &str) -> anyhow::Error {
    RevotingRefusal(message.to_string()).into()
}

/// Sets the number of allowed votes of the election to one in the caller's
/// transaction and queues its electoral-log record, which names `user_id`.
/// Returns the number it replaced.
///
/// # Errors
///
/// A [`RevotingRefusal`] when the election is not in the election event,
/// outside Preparation, during a lockdown, once voting has started on any
/// channel of the election event, or when re-voting is already disabled;
/// any other error is a database failure.
pub async fn disable_revoting(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    user_id: &str,
    username: Option<&str>,
) -> Result<i32> {
    // The status changes take these row locks first: no channel opens
    // between the checks below and the commit.
    lock_status_rows(tx, tenant_id, election_event_id, None).await?;
    let current: Option<i32> = tx
        .query_opt(
            "SELECT num_allowed_revotes FROM sequent_backend.election
             WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid
               AND id = $3::text::uuid",
            &[&tenant_id, &election_event_id, &election_id],
        )
        .await
        .context("Error reading the election's allowed votes")?
        .ok_or_else(|| refusal("The election is not part of this election event."))?
        .try_get(0)?;
    election_lifecycle::require_phase(
        tx,
        tenant_id,
        election_event_id,
        ElectoralPhase::Preparation,
    )
    .await
    .map_err(|error| refusal(&error.to_string()))?;
    if is_election_event_locked_down_in(tx, tenant_id, election_event_id).await? {
        return Err(refusal(
            "The election event is locked down; its re-voting setting can't change.",
        ));
    }
    let started: bool = tx
        .query_opt(VOTING_STARTED, &[&tenant_id, &election_event_id])
        .await
        .context("Error reading the voting status")?
        .context("Election event does not exist")?
        .try_get(0)?;
    if started {
        return Err(refusal(
            "Voting has started on a channel of this election event; re-voting can no longer be disabled.",
        ));
    }
    let previous = match current {
        Some(limit) if allows_revoting(current) => limit,
        _ => return Err(refusal("Re-voting is already disabled for this election.")),
    };
    tx.execute(
        "UPDATE sequent_backend.election SET num_allowed_revotes = $4
         WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid
           AND id = $3::text::uuid",
        &[&tenant_id, &election_event_id, &election_id, &SINGLE_VOTE],
    )
    .await
    .context("Error disabling re-voting")?;
    election_lifecycle::notice(
        tx,
        tenant_id,
        election_event_id,
        REVOTING_DISABLED,
        json!({
            "election_id": election_id,
            "num_allowed_revotes": {"from": previous, "to": SINGLE_VOTE},
            "disabled_by": user_id,
            "username": username,
        }),
    )
    .await?;
    Ok(previous)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unlimited_or_several_votes_allow_revoting() {
        assert!(allows_revoting(Some(0)));
        assert!(allows_revoting(Some(2)));
        assert!(allows_revoting(Some(30)));
        assert!(!allows_revoting(Some(SINGLE_VOTE)));
        assert!(!allows_revoting(None));
        assert!(!allows_revoting(Some(-1)));
    }
}
