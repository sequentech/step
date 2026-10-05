// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Cast votes read for voters, administrators and reports from wherever their
//! election event stores them: Hasura's `cast_vote` table or the event's ballot box.

use crate::postgres::cast_vote::count_unresolved_cast_votes;
use crate::services::ballot_box::BallotBoxPolicy;
use crate::services::cast_votes::CastVoteStatus;
use crate::services::election_event_board::{get_ballot_box_policy, get_election_event_board};
use crate::services::protocol_manager::get_electoral_log_store;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box::BallotStatus;
use electoral_log::adapters::ballot_box_reads::{BallotIdMatch, StoredBallot};
use electoral_log::adapters::postgres::PostgresStore;
use serde::Serialize;
use serde_json::Value;
use tokio_postgres::types::ToSql;
use tracing::instrument;
use uuid::Uuid;

/// Where an election event's cast votes are read from.
pub enum CastVoteSource {
    /// Hasura's `cast_vote` table.
    CastVoteTable,
    /// The ballot box in the electoral-log database of the event's board.
    BallotBox { store: PostgresStore, board: String },
}

/// Where an election event's cast votes are read from, by its ballot box policy.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_cast_vote_source(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<CastVoteSource> {
    let row = hasura_transaction
        .query_opt(
            "SELECT bulletin_board_reference FROM sequent_backend.election_event \
             WHERE tenant_id = $1::text::uuid AND id = $2::text::uuid",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the election event's ballot box policy")?
        .ok_or_else(|| anyhow!("Election event {election_event_id} not found"))?;
    let reference: Option<Value> = row.try_get(0)?;
    match get_ballot_box_policy(reference.clone()) {
        BallotBoxPolicy::CastVoteTable => Ok(CastVoteSource::CastVoteTable),
        BallotBoxPolicy::ElectoralLog => {
            let board = get_election_event_board(reference)
                .context("Election event has no electoral-log board")?;
            Ok(CastVoteSource::BallotBox {
                store: get_electoral_log_store(&board).await?,
                board,
            })
        }
    }
}

/// Votes of an election's area whose outcome is unresolved, which the tally refuses
/// to count: Datafix's `in-progress` cast votes, or `pending` ballots.
#[instrument(skip(hasura_transaction), err)]
pub async fn count_unresolved_votes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
) -> Result<i64> {
    match get_cast_vote_source(
        hasura_transaction,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
    )
    .await?
    {
        CastVoteSource::CastVoteTable => {
            count_unresolved_cast_votes(
                hasura_transaction,
                tenant_id,
                election_event_id,
                election_id,
                area_id,
            )
            .await
        }
        CastVoteSource::BallotBox { store, .. } => {
            store
                .pending_status_count(
                    &election_event_id.to_string(),
                    &election_id.to_string(),
                    &area_id.to_string(),
                )
                .await
        }
    }
}

/// The `cast_vote` status a stored ballot is shown with.
pub fn cast_vote_status(status: BallotStatus) -> CastVoteStatus {
    match status {
        BallotStatus::Valid => CastVoteStatus::Valid,
        BallotStatus::Pending => CastVoteStatus::InProgress,
        BallotStatus::Rejected => CastVoteStatus::Discarded,
    }
}

/// A time the ballot box gives in microseconds since the epoch.
pub fn ballot_box_time(micros: i64) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp_micros(micros)
        .ok_or_else(|| anyhow!("Ballot box time out of range: {micros}"))
}

/// A voter's cast vote, as the voting portal reads it.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct VoterCastVote {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub ballot_id: Option<String>,
    pub status: CastVoteStatus,
    /// RFC 3339.
    pub created_at: Option<String>,
    /// Only when looking up ballots by ID.
    pub content: Option<String>,
}

/// A voter as their token scopes them: one event, one area and some elections.
#[derive(Debug, Clone, Copy)]
pub struct VoterScope<'a> {
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub voter_id: &'a str,
    pub area_id: &'a str,
    pub election_ids: &'a [String],
}

/// Which of a voter's cast votes to read.
#[derive(Debug, Clone, Copy)]
pub enum VoterCastVotes<'a> {
    /// All of them, without their content.
    All,
    /// Those of an election whose ballot ID matches, with their content.
    Matching {
        election_id: &'a str,
        ballot_id: BallotIdMatch<'a>,
    },
}

/// A voter's cast votes in their area and authorized elections, in the order they
/// were cast: the rows Hasura's `user` role may read from `cast_vote`.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_voter_cast_votes(
    hasura_transaction: &Transaction<'_>,
    scope: &VoterScope<'_>,
    which: VoterCastVotes<'_>,
) -> Result<Vec<VoterCastVote>> {
    if let VoterCastVotes::Matching { election_id, .. } = which {
        if !scope.election_ids.iter().any(|id| id == election_id) {
            return Ok(Vec::new());
        }
    }
    match get_cast_vote_source(hasura_transaction, scope.tenant_id, scope.election_event_id).await?
    {
        CastVoteSource::CastVoteTable => {
            cast_vote_table_voter_votes(hasura_transaction, scope, which).await
        }
        CastVoteSource::BallotBox { store, .. } => {
            ballot_box_voter_votes(&store, scope, which).await
        }
    }
}

async fn cast_vote_table_voter_votes(
    hasura_transaction: &Transaction<'_>,
    scope: &VoterScope<'_>,
    which: VoterCastVotes<'_>,
) -> Result<Vec<VoterCastVote>> {
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![
        &scope.tenant_id,
        &scope.election_event_id,
        &scope.voter_id,
        &scope.area_id,
        &scope.election_ids,
    ];
    let (filter, content) = match &which {
        VoterCastVotes::All => ("", "NULL::text"),
        VoterCastVotes::Matching {
            election_id,
            ballot_id,
        } => {
            params.push(election_id);
            match ballot_id {
                BallotIdMatch::Exact(ballot_id) => {
                    params.push(ballot_id);
                    (
                        "AND election_id = $6::text::uuid AND ballot_id = $7",
                        "content",
                    )
                }
                BallotIdMatch::Prefix(prefix) => {
                    params.push(prefix);
                    (
                        "AND election_id = $6::text::uuid AND starts_with(ballot_id, $7)",
                        "content",
                    )
                }
            }
        }
    };
    let rows = hasura_transaction
        .query(
            &format!(
                "SELECT id::text, election_id::text, area_id::text, ballot_id, status, \
                        created_at, {content} \
                 FROM sequent_backend.cast_vote \
                 WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid \
                   AND voter_id_string = $3 AND area_id = $4::text::uuid \
                   AND election_id = ANY($5::text[]::uuid[]) {filter} \
                 ORDER BY created_at, id"
            ),
            &params,
        )
        .await
        .context("Error reading the voter's cast votes")?;
    rows.into_iter()
        .map(|row| {
            let status: String = row.try_get(4)?;
            let created_at: Option<DateTime<Utc>> = row.try_get(5)?;
            Ok(VoterCastVote {
                id: row.try_get(0)?,
                tenant_id: scope.tenant_id.to_string(),
                election_event_id: scope.election_event_id.to_string(),
                election_id: row.try_get(1)?,
                area_id: row.try_get(2)?,
                ballot_id: row.try_get(3)?,
                status: status
                    .parse()
                    .map_err(|_| anyhow!("Invalid cast vote status {status:?}"))?,
                created_at: created_at.map(|time| time.to_rfc3339()),
                content: row.try_get(6)?,
            })
        })
        .collect()
}

fn voter_cast_vote(
    scope: &VoterScope<'_>,
    ballot: StoredBallot,
    content: Option<String>,
) -> Result<VoterCastVote> {
    Ok(VoterCastVote {
        id: ballot.id,
        tenant_id: scope.tenant_id.to_string(),
        election_event_id: scope.election_event_id.to_string(),
        election_id: Some(ballot.election_id),
        area_id: Some(ballot.area_id),
        ballot_id: Some(ballot.ballot_id),
        status: cast_vote_status(ballot.status),
        created_at: Some(ballot_box_time(ballot.accepted_at)?.to_rfc3339()),
        content,
    })
}

fn in_scope(scope: &VoterScope<'_>, ballot: &StoredBallot) -> bool {
    ballot.area_id == scope.area_id && scope.election_ids.contains(&ballot.election_id)
}

async fn ballot_box_voter_votes(
    store: &PostgresStore,
    scope: &VoterScope<'_>,
    which: VoterCastVotes<'_>,
) -> Result<Vec<VoterCastVote>> {
    match which {
        VoterCastVotes::All => store
            .voter_ballots(scope.election_event_id, scope.voter_id)
            .await?
            .into_iter()
            .filter(|ballot| in_scope(scope, ballot))
            .map(|ballot| voter_cast_vote(scope, ballot, None))
            .collect(),
        VoterCastVotes::Matching {
            election_id,
            ballot_id,
        } => store
            .voter_ballot_contents(
                scope.election_event_id,
                scope.voter_id,
                election_id,
                ballot_id,
            )
            .await?
            .into_iter()
            .filter(|found| in_scope(scope, &found.ballot))
            .map(|found| voter_cast_vote(scope, found.ballot, Some(found.content)))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ballot_statuses_read_as_cast_vote_statuses() {
        assert_eq!(cast_vote_status(BallotStatus::Valid), CastVoteStatus::Valid);
        assert_eq!(
            cast_vote_status(BallotStatus::Pending),
            CastVoteStatus::InProgress
        );
        assert_eq!(
            cast_vote_status(BallotStatus::Rejected),
            CastVoteStatus::Discarded
        );
    }

    #[test]
    fn ballot_box_times_are_microseconds() {
        assert_eq!(
            ballot_box_time(1_791_230_400_123_456).unwrap().to_rfc3339(),
            "2026-10-05T20:00:00.123456+00:00"
        );
        assert!(ballot_box_time(i64::MAX).is_err());
    }

    fn ballot(area_id: &str, election_id: &str) -> StoredBallot {
        StoredBallot {
            id: "id".into(),
            ballot_id: "ballot".into(),
            election_id: election_id.into(),
            area_id: area_id.into(),
            voter_id: "voter".into(),
            status: BallotStatus::Valid,
            voting_channel: "ONLINE".into(),
            accepted_at: 0,
        }
    }

    #[test]
    fn voters_read_only_ballots_of_their_area_and_elections() {
        let elections = vec!["e1".to_string()];
        let scope = VoterScope {
            tenant_id: "t",
            election_event_id: "ev",
            voter_id: "voter",
            area_id: "a1",
            election_ids: &elections,
        };
        assert!(in_scope(&scope, &ballot("a1", "e1")));
        assert!(!in_scope(&scope, &ballot("a2", "e1")));
        assert!(!in_scope(&scope, &ballot("a1", "e2")));
    }
}
