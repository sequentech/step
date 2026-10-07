// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Changes of a ballot's status after it was accepted, for Datafix events. Their
//! ballots are accepted as pending; Datafix's answer makes them valid or rejected,
//! and disabling a voter rejects the voter's ballots. A rejected ballot no longer
//! counts toward the voter's votes, so the voter may vote again. The records of
//! these operations are appended to the log by the Datafix code that runs them.

use super::ballot_box::BallotStatus;
use super::ballot_box_reads::StoredBallot;
use super::postgres::PostgresStore;
use anyhow::{anyhow, ensure, Context, Result};

/// Which of a voter's ballots in an election event are pending or valid.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VoterBallotState {
    /// At least one ballot waits for its outcome.
    pub has_pending: bool,
    /// At least one ballot is valid.
    pub has_valid: bool,
}

/// A ballot whose outcome is pending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BallotToReview {
    pub election_event_id: String,
    /// The ballot's ID in the ballot box, as returned when it was accepted.
    pub id: String,
}

/// `BallotStatus::Pending`, written into queries so that they use the partial index
/// of pending ballots, whose condition must match the query's.
const PENDING: &str = "pending";

const PENDING_BALLOT: &str = "SELECT id::text, ballot_id, election_id::text, area_id::text, \
     voter_id, status, voting_channel, (extract(epoch FROM accepted_at) * 1000000)::bigint \
     FROM ballot_box_ballot \
     WHERE election_event_id = $1::text::uuid AND id = $2::text::uuid AND status = 'pending'";

/// Changes a ballot's status if it still has the expected one. A ballot that
/// becomes rejected gives the voter's vote back. The voter's ID lets the lookup use
/// the index of each voter's ballots.
const SET_STATUS: &str = r#"
WITH changed AS (
    UPDATE ballot_box_ballot SET status = $5
    WHERE election_event_id = $1::text::uuid AND voter_id = $2 AND id = $3::text::uuid
      AND status = $4
    RETURNING election_event_id, election_id, voter_id
), released AS (
    UPDATE ballot_box_voter v SET votes = v.votes - 1, updated_at = now()
    FROM changed c
    WHERE $5 = $6
      AND v.election_event_id = c.election_event_id
      AND v.election_id = c.election_id
      AND v.voter_id = c.voter_id
)
SELECT count(*) FROM changed
"#;

/// Rejects a voter's pending and valid ballots in an event and gives back their
/// votes, election by election.
const REJECT_VOTER: &str = r#"
WITH rejected AS (
    UPDATE ballot_box_ballot SET status = $3
    WHERE election_event_id = $1::text::uuid AND voter_id = $2 AND status = ANY($4)
    RETURNING election_event_id, election_id, voter_id
), per_election AS (
    SELECT election_event_id, election_id, voter_id, count(*) AS ballots
    FROM rejected GROUP BY 1, 2, 3
), released AS (
    UPDATE ballot_box_voter v SET votes = v.votes - p.ballots, updated_at = now()
    FROM per_election p
    WHERE v.election_event_id = p.election_event_id
      AND v.election_id = p.election_id
      AND v.voter_id = p.voter_id
)
SELECT count(*) FROM rejected
"#;

fn stored_ballot(row: &tokio_postgres::Row) -> Result<StoredBallot> {
    let status: String = row.try_get(5)?;
    Ok(StoredBallot {
        id: row.try_get(0)?,
        ballot_id: row.try_get(1)?,
        election_id: row.try_get(2)?,
        area_id: row.try_get(3)?,
        voter_id: row.try_get(4)?,
        status: status
            .parse()
            .map_err(|_| anyhow!("Unknown ballot status {status:?}"))?,
        voting_channel: row.try_get(6)?,
        accepted_at: row.try_get(7)?,
    })
}

impl PostgresStore {
    /// A ballot of the event whose outcome is pending, by its ID; `None` when there
    /// is no such ballot or its outcome is known.
    pub async fn pending_ballot(
        &self,
        election_event_id: &str,
        id: &str,
    ) -> Result<Option<StoredBallot>> {
        let row = self
            .client()
            .await?
            .query_opt(PENDING_BALLOT, &[&election_event_id, &id])
            .await
            .context("Error reading a pending ballot")?;
        row.as_ref().map(stored_ballot).transpose()
    }

    /// Change a ballot's status from `expected` to `next`. Returns false, and
    /// changes nothing, when the ballot does not have the expected status, for
    /// example because another run already changed it. A rejected ballot's status
    /// does not change any more.
    pub async fn set_ballot_status(
        &self,
        election_event_id: &str,
        voter_id: &str,
        id: &str,
        expected: BallotStatus,
        next: BallotStatus,
    ) -> Result<bool> {
        ensure!(
            expected != BallotStatus::Rejected,
            "A rejected ballot's status does not change"
        );
        let (expected, next, rejected) = (
            expected.to_string(),
            next.to_string(),
            BallotStatus::Rejected.to_string(),
        );
        let row = self
            .client()
            .await?
            .query_one(
                SET_STATUS,
                &[
                    &election_event_id,
                    &voter_id,
                    &id,
                    &expected,
                    &next,
                    &rejected,
                ],
            )
            .await
            .context("Error changing a ballot's status")?;
        let changed: i64 = row.try_get(0)?;
        Ok(changed > 0)
    }

    /// Reject a voter's pending and valid ballots in an event. Returns how many it
    /// rejected.
    pub async fn reject_voter_ballots(
        &self,
        election_event_id: &str,
        voter_id: &str,
    ) -> Result<i64> {
        let rejected = BallotStatus::Rejected.to_string();
        let active = vec![
            BallotStatus::Pending.to_string(),
            BallotStatus::Valid.to_string(),
        ];
        let row = self
            .client()
            .await?
            .query_one(
                REJECT_VOTER,
                &[&election_event_id, &voter_id, &rejected, &active],
            )
            .await
            .context("Error rejecting the voter's ballots")?;
        Ok(row.try_get(0)?)
    }

    /// Whether a voter has pending or valid ballots in an event.
    pub async fn voter_ballot_state(
        &self,
        election_event_id: &str,
        voter_id: &str,
    ) -> Result<VoterBallotState> {
        let (pending, valid) = (
            BallotStatus::Pending.to_string(),
            BallotStatus::Valid.to_string(),
        );
        let row = self
            .client()
            .await?
            .query_one(
                "SELECT coalesce(bool_or(status = $3), false), coalesce(bool_or(status = $4), false) \
                 FROM ballot_box_ballot \
                 WHERE election_event_id = $1::text::uuid AND voter_id = $2",
                &[&election_event_id, &voter_id, &pending, &valid],
            )
            .await
            .context("Error reading the voter's ballot state")?;
        Ok(VoterBallotState {
            has_pending: row.try_get(0)?,
            has_valid: row.try_get(1)?,
        })
    }

    /// The state of every voter with a pending or valid ballot in an event.
    pub async fn voter_ballot_states(
        &self,
        election_event_id: &str,
    ) -> Result<Vec<(String, VoterBallotState)>> {
        let (pending, valid) = (
            BallotStatus::Pending.to_string(),
            BallotStatus::Valid.to_string(),
        );
        let rows = self
            .client()
            .await?
            .query(
                "SELECT voter_id, bool_or(status = $2), bool_or(status = $3) \
                 FROM ballot_box_ballot \
                 WHERE election_event_id = $1::text::uuid AND status IN ($2, $3) \
                 GROUP BY voter_id",
                &[&election_event_id, &pending, &valid],
            )
            .await
            .context("Error reading the voters' ballot states")?;
        rows.iter()
            .map(|row| {
                Ok((
                    row.try_get(0)?,
                    VoterBallotState {
                        has_pending: row.try_get(1)?,
                        has_valid: row.try_get(2)?,
                    },
                ))
            })
            .collect()
    }

    /// Pending ballots of every event in the database accepted at least
    /// `min_age_secs` seconds ago, in `(event, ID)` order, after the given one.
    pub async fn ballots_to_review(
        &self,
        after: Option<&BallotToReview>,
        min_age_secs: f64,
        limit: i64,
    ) -> Result<Vec<BallotToReview>> {
        let (after_event, after_id) = match after {
            Some(ballot) => (
                Some(ballot.election_event_id.as_str()),
                Some(ballot.id.as_str()),
            ),
            None => (None, None),
        };
        let rows = self
            .client()
            .await?
            .query(
                &format!(
                    "SELECT election_event_id::text, id::text FROM ballot_box_ballot \
                     WHERE status = '{PENDING}' \
                       AND accepted_at < now() - make_interval(secs => $3) \
                       AND ($1::text IS NULL \
                            OR (election_event_id, id) > ($1::text::uuid, $2::text::uuid)) \
                     ORDER BY election_event_id, id \
                     LIMIT $4"
                ),
                &[&after_event, &after_id, &min_age_secs, &limit],
            )
            .await
            .context("Error listing the ballots to review")?;
        rows.iter()
            .map(|row| {
                Ok(BallotToReview {
                    election_event_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_name_the_pending_status_as_the_partial_index_does() {
        assert_eq!(PENDING, BallotStatus::Pending.to_string());
        assert!(PENDING_BALLOT.ends_with(&format!("status = '{PENDING}'")));
        assert!(include_str!("../../schema.sql").contains(&format!(
            "ON ballot_box_ballot (election_event_id, id) WHERE status = '{PENDING}'"
        )));
    }
}
