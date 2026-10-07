// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Cast votes stored in the electoral log's ballot box.
//!
//! Each election event has a ballot box policy: events created before the ballot
//! box keep storing cast votes in Hasura's `cast_vote` table, and new events store
//! them in the ballot box of their electoral-log database.

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::external::utils::DATAFIX_ID_KEY;
use crate::services::protocol_manager::{
    get_electoral_log_router, get_electoral_log_store, get_protocol_manager,
};
use anyhow::{Context, Result};
use b4::messages::message::Signer;
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box::{
    AcceptBallot, AcceptOutcome, BallotStatus, PendingBallot,
};
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::messages::message::Message;
use electoral_log::messages::newtypes::{
    CastVoteHash, ElectionIdString, EventIdString, PseudonymHash, VoterCountryString,
    VoterIpString, VotingChannelString,
};
use electoral_log::ports::ElectoralLogStore;
use electoral_log::LogEntry;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use strand::backend::ristretto::RistrettoCtx;
use strand::hash::Hash;
use strand::signature::StrandSignatureSk;
use strum_macros::{Display, EnumString};
use tracing::instrument;

/// Most ballots one sequencer append covers.
pub const SEQUENCER_BATCH: i64 = 5_000;
/// How long one sequencer run keeps appending before leaving the rest to the next.
const SEQUENCER_BUDGET: Duration = Duration::from_secs(50);
/// Seconds a sequencer run holds an event; longer than any run, so that a run
/// that dies leaves the event to the next one after this delay at most.
const SEQUENCER_LEASE_SECS: i32 = 150;

/// How long the tally waits for the sequencer to append an area's accepted ballots.
pub const TALLY_SEQUENCER_WAIT: Duration = Duration::from_secs(60);
/// How long the checkpoint of voting's close waits for the sequencer.
pub const CLOSING_SEQUENCER_WAIT: Duration = Duration::from_secs(60);
const SEQUENCER_POLL: Duration = Duration::from_secs(1);

/// Wait until `waiting`, a count of accepted ballots the sequencer has not appended
/// yet, reaches 0 or `timeout` passes. Returns the last count.
pub async fn wait_for_sequencer<F, Fut>(timeout: Duration, mut waiting: F) -> Result<i64>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<i64>>,
{
    let started = Instant::now();
    loop {
        let count = waiting().await?;
        if count == 0 || started.elapsed() >= timeout {
            return Ok(count);
        }
        tokio::time::sleep(SEQUENCER_POLL.min(timeout)).await;
    }
}

/// Where an election event's cast votes are stored.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Display, EnumString, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum BallotBoxPolicy {
    /// Hasura's `cast_vote` table; events created before the ballot box.
    #[default]
    CastVoteTable,
    /// The ballot box of the event's electoral-log database.
    ElectoralLog,
}

/// Where a new election event stores its cast votes: the ballot box, except for a
/// Datafix event, which keeps `cast_vote` until Datafix's outcomes are recorded in
/// the ballot box.
pub fn new_event_ballot_box_policy(annotations: Option<&serde_json::Value>) -> BallotBoxPolicy {
    let datafix = annotations
        .and_then(|annotations| annotations.get(DATAFIX_ID_KEY))
        .is_some();
    if datafix {
        BallotBoxPolicy::CastVoteTable
    } else {
        BallotBoxPolicy::ElectoralLog
    }
}

/// How a stored ballot's content is encoded, so that formats can coexist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum BallotFormat {
    /// A `HashableBallot`: one ciphertext per contest.
    HashableBallot,
    /// A `HashableMultiBallot`: all contests in one ciphertext.
    HashableMultiBallot,
}

/// Votes a voter may cast in an election, as `check_revote_limit` computes them for
/// `cast_vote`: `num_allowed_revotes`, 1 when unset, and 0 for unlimited.
pub async fn get_allowed_votes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<i32> {
    let row = hasura_transaction
        .query_opt(
            r#"
            SELECT num_allowed_revotes
            FROM sequent_backend.election
            WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid
              AND id = $3::text::uuid
            "#,
            &[&tenant_id, &election_event_id, &election_id],
        )
        .await
        .context("Error reading the election's allowed revotes")?;
    let allowed: Option<i32> = match row {
        Some(row) => row
            .try_get(0)
            .context("Error reading the election's allowed revotes")?,
        None => None,
    };
    Ok(allowed.unwrap_or(1))
}

/// Accept a vote into the ballot box of the board's database.
pub async fn accept_ballot(board: &str, ballot: &AcceptBallot<'_>) -> Result<AcceptOutcome> {
    get_electoral_log_store(board)
        .await?
        .accept_ballot(ballot)
        .await
}

/// Status of a newly accepted ballot.
pub fn initial_ballot_status(datafix: bool) -> BallotStatus {
    if datafix {
        BallotStatus::Pending
    } else {
        BallotStatus::Valid
    }
}

/// Delivery ID of an accepted ballot's cast-vote record, so that appending it again
/// stores nothing new.
pub fn ballot_delivery_id(election_event_id: &str, seq: i64) -> String {
    format!("ballot-box:{election_event_id}:{seq}")
}

fn stored_hash(bytes: &[u8]) -> Result<Hash> {
    bytes
        .try_into()
        .context("A stored ballot hash does not have 64 bytes")
}

/// The cast-vote record of an accepted ballot, built as `post_cast_vote` builds the
/// record of a vote stored in `cast_vote`.
pub fn ballot_record(
    board: &str,
    election_event_id: &str,
    ballot: &PendingBallot,
    signing_key: &StrandSignatureSk,
) -> Result<LogEntry> {
    let log = ElectoralLog::for_voter_with_signing_key(board, &ballot.voter_id, signing_key);
    let message = Message::cast_vote_with_channel_message(
        EventIdString(election_event_id.to_string()),
        ElectionIdString(Some(ballot.election_id.clone())),
        PseudonymHash::new(stored_hash(&ballot.pseudonym_hash)?),
        CastVoteHash::new(stored_hash(&ballot.ballot_hash)?),
        &log.sd,
        VoterIpString(format!("ip: {}", ballot.voter_ip.as_deref().unwrap_or(""))),
        VoterCountryString(format!(
            "country: {}",
            ballot.voter_country.as_deref().unwrap_or("")
        )),
        VotingChannelString(ballot.voting_channel.clone()),
        Some(ballot.voter_id.clone()),
        ballot.username.clone(),
        ballot.area_id.clone(),
    )?;
    Ok(LogEntry {
        delivery_id: ballot_delivery_id(election_event_id, ballot.seq),
        message: (&message).try_into()?,
    })
}

/// Append the records of an event's accepted ballots to its board, oldest first,
/// until none wait or the run's time budget is spent. Returns how many it appended,
/// or `None` when another sequencer is working on the event.
#[instrument(err)]
pub async fn sequence_event(tenant_id: &str, election_event_id: &str) -> Result<Option<usize>> {
    let (board, signing_key) = {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.build_transaction().read_only(true).start().await?;
        let event = get_election_event_by_id(&transaction, tenant_id, election_event_id).await?;
        let board = get_election_event_board(event.bulletin_board_reference)
            .context("Election event has no electoral-log board")?;
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(
            &transaction,
            tenant_id,
            Some(election_event_id),
            &board,
        )
        .await?;
        (board, protocol_manager.get_signing_key().clone())
    };
    let store = get_electoral_log_store(&board).await?;
    let holder = uuid::Uuid::new_v4().to_string();
    if !store
        .take_sequencer_lease(election_event_id, &holder, SEQUENCER_LEASE_SECS)
        .await?
    {
        return Ok(None);
    }
    let appended = append_pending(&store, &board, election_event_id, &signing_key).await;
    store
        .release_sequencer_lease(election_event_id, &holder)
        .await?;
    let appended = appended?;
    Ok(Some(appended))
}

async fn append_pending(
    store: &PostgresStore,
    board: &str,
    election_event_id: &str,
    signing_key: &StrandSignatureSk,
) -> Result<usize> {
    let started = Instant::now();
    let mut appended = 0;
    loop {
        let pending = store
            .pending_ballots(election_event_id, SEQUENCER_BATCH)
            .await?;
        if pending.is_empty() {
            break;
        }
        let entries = pending
            .iter()
            .map(|ballot| ballot_record(board, election_event_id, ballot, signing_key))
            .collect::<Result<Vec<_>>>()?;
        store
            .append(board, &mut entries.into_iter().map(anyhow::Ok))
            .await?;
        let seqs: Vec<i64> = pending.iter().map(|ballot| ballot.seq).collect();
        store.remove_pending(election_event_id, &seqs).await?;
        appended += seqs.len();
        if started.elapsed() > SEQUENCER_BUDGET {
            break;
        }
    }
    Ok(appended)
}

/// `(tenant_id, election_event_id)` of every event whose ballot box holds ballots
/// waiting for the sequencer, in every electoral-log database. A database that
/// cannot be read is logged and skipped.
#[instrument(err)]
pub async fn events_waiting_for_sequencer() -> Result<Vec<(String, String)>> {
    let router = get_electoral_log_router().await?;
    let mut stores = vec![router.shared()];
    for database in router.tenant_databases().await? {
        stores.push(router.database_store(&database).await?);
    }
    let mut events = Vec::new();
    for store in stores {
        match store.events_with_pending_ballots().await {
            Ok(found) => events.extend(found),
            Err(error) => tracing::warn!("Skipping an electoral-log database: {error:#}"),
        }
    }
    if events.is_empty() {
        return Ok(Vec::new());
    }
    let mut client = get_hasura_pool().await.get().await?;
    let transaction = client.build_transaction().read_only(true).start().await?;
    let rows = transaction
        .query(
            "SELECT tenant_id::text, id::text FROM sequent_backend.election_event \
             WHERE id = ANY($1::text[]::uuid[])",
            &[&events],
        )
        .await
        .context("Error reading the tenants of the events to sequence")?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn waiting_for_the_sequencer_ends_when_it_catches_up_or_times_out() {
        let mut counts: Vec<i64> = vec![0, 2, 5];
        let left = wait_for_sequencer(Duration::from_secs(5), || {
            let count = counts.pop().unwrap_or(0);
            async move { Ok(count) }
        })
        .await
        .unwrap();
        assert_eq!(left, 0);
        let left = wait_for_sequencer(Duration::ZERO, || async { Ok(7) })
            .await
            .unwrap();
        assert_eq!(left, 7);
        assert!(wait_for_sequencer(Duration::ZERO, || async {
            Err(anyhow::anyhow!("unreachable"))
        })
        .await
        .is_err());
    }

    #[test]
    fn new_events_use_the_ballot_box_except_datafix_events() {
        assert_eq!(
            new_event_ballot_box_policy(None),
            BallotBoxPolicy::ElectoralLog
        );
        assert_eq!(
            new_event_ballot_box_policy(Some(&serde_json::json!({"other": "x"}))),
            BallotBoxPolicy::ElectoralLog
        );
        assert_eq!(
            new_event_ballot_box_policy(Some(&serde_json::json!({ DATAFIX_ID_KEY: "event" }))),
            BallotBoxPolicy::CastVoteTable
        );
    }

    #[test]
    fn events_without_a_policy_keep_the_cast_vote_table() {
        assert_eq!(BallotBoxPolicy::default(), BallotBoxPolicy::CastVoteTable);
        assert_eq!(
            serde_json::to_value(BallotBoxPolicy::ElectoralLog).unwrap(),
            serde_json::json!("electoral-log")
        );
        assert_eq!(
            serde_json::from_value::<BallotBoxPolicy>(serde_json::json!("cast-vote-table"))
                .unwrap(),
            BallotBoxPolicy::CastVoteTable
        );
    }

    #[test]
    fn formats_have_stable_names() {
        assert_eq!(BallotFormat::HashableBallot.to_string(), "hashable-ballot");
        assert_eq!(
            BallotFormat::HashableMultiBallot.to_string(),
            "hashable-multi-ballot"
        );
    }

    /// The sequencer against a real server, in the database of
    /// `ELECTORAL_LOG_TEST_DATABASE_URL`.
    #[tokio::test]
    #[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
    async fn the_sequencer_appends_each_accepted_ballot_once_in_order() -> Result<()> {
        let config = std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?;
        let store = PostgresStore::new(config)?;
        store.initialize().await?;
        let event = uuid::Uuid::new_v4().to_string();
        let election = uuid::Uuid::new_v4().to_string();
        let area = uuid::Uuid::new_v4().to_string();
        let board = format!("test-ballot-box-{event}");
        store.create_board(&board).await?;
        store.create_ballot_box(&event).await?;
        let key = StrandSignatureSk::generate()?;
        for (index, voter) in ["v1", "v2", "v3"].iter().enumerate() {
            let ballot_id = format!("{index:064x}");
            let outcome = store
                .accept_ballot(&AcceptBallot {
                    election_event_id: &event,
                    election_id: &election,
                    area_id: &area,
                    voter_id: voter,
                    ballot_id: &ballot_id,
                    format: "hashable-ballot",
                    content: "ciphertext",
                    voter_signature: None,
                    pseudonym_hash: &[index as u8; 64],
                    ballot_hash: &[9; 64],
                    voting_channel: "ONLINE",
                    status: BallotStatus::Valid,
                    voter_ip: Some("192.0.2.1"),
                    voter_country: None,
                    username: Some(voter),
                    allowed_votes: 1,
                })
                .await?;
            assert!(matches!(outcome, AcceptOutcome::Accepted { .. }));
        }

        assert_eq!(append_pending(&store, &board, &event, &key).await?, 3);
        assert_eq!(store.pending_count(&event).await?, 0);
        // Nothing waits, so a second run appends nothing.
        assert_eq!(append_pending(&store, &board, &event, &key).await?, 0);

        let records = store
            .query(&board, &electoral_log::LogQuery::default())
            .await?;
        assert_eq!(records.len(), 3);
        assert!(records
            .iter()
            .all(|record| record.statement_kind == "CastVote"));
        let users: Vec<_> = records
            .iter()
            .map(|record| record.user_id.clone())
            .collect();
        assert!(users.contains(&Some("v1".to_string())) && users.contains(&Some("v3".to_string())));

        // A record appended twice is stored once.
        let again = store.pending_ballots(&event, 10).await?;
        assert!(again.is_empty());
        let report = store.audit(&board, &[]).await?;
        assert!(report.is_clean(), "{:?}", report.findings());

        store.delete_board(&board).await?;
        let suffix = event.replace('-', "");
        store
            .client()
            .await?
            .batch_execute(&format!(
                "DROP TABLE ballot_box_ballot_{suffix}, ballot_box_voter_{suffix};"
            ))
            .await?;
        Ok(())
    }
}
