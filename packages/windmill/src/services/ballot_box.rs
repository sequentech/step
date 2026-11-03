// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Cast votes, stored in each election event's ballot box in the electoral-log
//! database.

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::ballot_box_reads::get_event_ballot_box;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::electoral_log_board::{
    get_board_signing_key, get_event_databases, get_event_store,
};
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box::{AcceptBallot, AcceptOutcome, PendingBallot};
use electoral_log::adapters::ballot_box_status::VoterBallotState;
use electoral_log::adapters::events::{ActivityMark, BallotActivity};
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::messages::message::Message;
use electoral_log::messages::newtypes::{
    CastVoteHash, ElectionIdString, EventIdString, PseudonymHash, VoterCountryString,
    VoterIpString, VotingChannelString,
};
use electoral_log::ports::ElectoralLogStore;
use electoral_log::LogEntry;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use strum_macros::{Display, EnumString};
use tracing::instrument;
use voting_crypto::hash::Hash;
use voting_crypto::signing::SigningKey;
use voting_crypto::RistrettoCtx;

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

/// How a stored ballot's content is encoded, so that formats can coexist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum BallotFormat {
    /// A `HashableBallot`: one ciphertext per contest.
    HashableBallot,
    /// A `HashableMultiBallot`: all contests in one ciphertext.
    HashableMultiBallot,
}

/// Votes a voter may cast in an election: `num_allowed_revotes`, 1 when unset, and 0
/// for unlimited.
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

/// Accept a vote into the event's ballot box. The event is marked first, so that the
/// sequencer and the review of pending votes visit it.
pub async fn accept_ballot(ballot: &AcceptBallot<'_>) -> Result<AcceptOutcome> {
    let databases = get_event_databases().await?;
    databases
        .mark_ballot_activity(ballot.election_event_id, ActivityMark::Throttled)
        .await?;
    databases
        .store(ballot.election_event_id)
        .await?
        .accept_ballot(ballot)
        .await
}

/// Mark an event so that the sequencer visits it at its next run, before waiting for
/// it.
pub async fn request_sequencer(election_event_id: &str) -> Result<()> {
    get_event_databases()
        .await?
        .mark_ballot_activity(election_event_id, ActivityMark::Immediate)
        .await
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

/// The cast-vote record of an accepted ballot, signed with the event's
/// protocol-manager key.
pub fn ballot_record(
    board: &str,
    election_event_id: &str,
    ballot: &PendingBallot,
    signing_key: &SigningKey,
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
        let protocol_manager =
            get_board_signing_key(&transaction, tenant_id, Some(election_event_id), &board).await?;
        (board, protocol_manager.clone())
    };
    let store = get_event_store(election_event_id).await?;
    let holder = uuid::Uuid::new_v4().to_string();
    if !store
        .take_sequencer_lease(election_event_id, &holder, SEQUENCER_LEASE_SECS)
        .await?
    {
        return Ok(None);
    }
    let appended = append_pending(&store, &board, tenant_id, election_event_id, &signing_key).await;
    store
        .release_sequencer_lease(election_event_id, &holder)
        .await?;
    let appended = appended?;
    Ok(Some(appended))
}

async fn append_pending(
    store: &PostgresStore,
    board: &str,
    tenant_id: &str,
    election_event_id: &str,
    signing_key: &SigningKey,
) -> Result<usize> {
    let started = Instant::now();
    let mut appended = 0;
    let private = store.ballot_identity(election_event_id).await?
        == electoral_log::adapters::privacy::BallotIdentity::Pseudonym;

    loop {
        // Received evidence is queued in the receive transaction. Drain it before
        // casts so a fast confirmation can never overtake its authority binding.
        let received = store
            .pending_received_ballots(election_event_id, SEQUENCER_BATCH)
            .await?;
        if !received.is_empty() {
            let entries = received
                .iter()
                .map(|item| {
                    crate::services::vote_privacy::received_log_entry(tenant_id, item, signing_key)
                })
                .collect::<Result<Vec<_>>>()?;
            store
                .append(board, &mut entries.into_iter().map(Ok))
                .await?;
            let ids: Vec<i64> = received.iter().map(|item| item.id).collect();
            store
                .remove_pending_received(election_event_id, &ids)
                .await?;
            appended += ids.len();
            if started.elapsed() > SEQUENCER_BUDGET {
                break;
            }
            continue;
        }
        let audit = store
            .pending_privacy_messages(election_event_id, SEQUENCER_BATCH)
            .await?;
        if !audit.is_empty() {
            use voting_crypto::RecordDeserialize;
            let entries = audit
                .iter()
                .map(|item| {
                    let message = Message::record_deserialize(&item.message)?;
                    Ok(LogEntry {
                        delivery_id: item.delivery_id.clone(),
                        message: (&message).try_into()?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            store
                .append(board, &mut entries.into_iter().map(Ok))
                .await?;
            store
                .remove_pending_privacy_messages(
                    election_event_id,
                    &audit.iter().map(|item| item.id).collect::<Vec<_>>(),
                )
                .await?;
            appended += audit.len();
        }
        let pending = store
            .pending_ballots(election_event_id, SEQUENCER_BATCH)
            .await?;
        if pending.is_empty() {
            break;
        }
        let mut entries = Vec::with_capacity(pending.len() * 2);
        let received: HashMap<(String, String, String), _> = if private {
            store
                .received_ballots_for_hashes(
                    election_event_id,
                    &pending
                        .iter()
                        .map(|b| b.ballot_id.clone())
                        .collect::<Vec<_>>(),
                )
                .await?
                .into_iter()
                .map(|r| {
                    (
                        (
                            r.election_id.clone(),
                            r.voter_id.clone(),
                            r.ballot_hash.clone(),
                        ),
                        r,
                    )
                })
                .collect()
        } else {
            HashMap::new()
        };
        for ballot in &pending {
            if private {
                let received = received
                    .get(&(
                        ballot.election_id.clone(),
                        ballot.voter_id.clone(),
                        ballot.ballot_id.clone(),
                    ))
                    .context("A pseudonymous cast requires its received ballot")?;
                // Even when receive/cast commit after our queue-empty check,
                // append the binding first. Stable delivery IDs deduplicate it
                // if it was already appended or its outbox is retried later.
                entries.push(crate::services::vote_privacy::received_log_entry(
                    tenant_id,
                    &electoral_log::adapters::privacy::PendingReceivedBallot {
                        id: 0,
                        queued_at: received.received_at,
                        kind: electoral_log::adapters::privacy::ReceivedEvidenceKind::Received,
                        ballot: received.clone(),
                    },
                    signing_key,
                )?);
                entries.push(crate::services::vote_privacy::cast_log_entry(
                    received,
                    tenant_id,
                    election_event_id,
                    ballot,
                    signing_key,
                )?);
            } else {
                entries.push(ballot_record(
                    board,
                    election_event_id,
                    ballot,
                    signing_key,
                )?);
            }
        }
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

/// Whether a voter has pending or valid votes in an election event.
pub async fn get_voter_ballot_state(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> Result<VoterBallotState> {
    let event = get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;
    let identity = if crate::services::vote_privacy::identity_mode(&event)?
        == sequent_core::types::vote_privacy::BallotIdentity::Pseudonym
    {
        let Some(code) = get_event_store(election_event_id)
            .await?
            .voter_pseudonym(election_event_id, voter_id)
            .await?
        else {
            return Ok(VoterBallotState {
                has_pending: false,
                has_valid: false,
            });
        };
        code
    } else {
        voter_id.to_owned()
    };
    get_event_ballot_box(hasura_transaction, tenant_id, election_event_id)
        .await?
        .store
        .voter_ballot_state(election_event_id, &identity)
        .await
}

/// The state of every voter with a pending or valid vote in an election event,
/// by voter ID.
pub async fn get_voter_ballot_states(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, VoterBallotState>> {
    let store = get_event_ballot_box(hasura_transaction, tenant_id, election_event_id)
        .await?
        .store;
    let states = store.voter_ballot_states(election_event_id).await?;
    if store.ballot_identity(election_event_id).await?
        != electoral_log::adapters::privacy::BallotIdentity::Pseudonym
    {
        return Ok(states.into_iter().collect());
    }
    let codes = states
        .iter()
        .map(|(code, _)| code.clone())
        .collect::<Vec<_>>();
    let voters = store
        .voters_for_pseudonyms(election_event_id, &codes)
        .await?;
    Ok(states
        .into_iter()
        .filter_map(|(code, state)| voters.get(&code).map(|voter| (voter.clone(), state)))
        .collect())
}

/// Internal messaging audience: private participation has no per-ballot time.
pub async fn get_voters_who_participated(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
) -> Result<Vec<String>> {
    let store = get_event_ballot_box(tx, tenant, event).await?.store;
    if store.ballot_identity(event).await?
        == electoral_log::adapters::privacy::BallotIdentity::Pseudonym
    {
        return store.participating_voters(event, None).await;
    }
    Ok(get_voter_ballot_states(tx, tenant, event)
        .await?
        .into_keys()
        .collect())
}

/// Reject a voter's pending and valid votes in an election event. Returns how
/// many it rejected.
pub async fn reject_voter_ballots(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> Result<i64> {
    let event = get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;
    let identity = if crate::services::vote_privacy::identity_mode(&event)?
        == sequent_core::types::vote_privacy::BallotIdentity::Pseudonym
    {
        let Some(code) = get_event_store(election_event_id)
            .await?
            .voter_pseudonym(election_event_id, voter_id)
            .await?
        else {
            return Ok(0);
        };
        code
    } else {
        voter_id.to_owned()
    };
    get_event_ballot_box(hasura_transaction, tenant_id, election_event_id)
        .await?
        .store
        .reject_voter_ballots(election_event_id, &identity)
        .await
}

/// What a marked event's ballot box holds for the background tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BallotBoxWork {
    /// Accepted ballots wait for the sequencer.
    Sequencing,
    /// Only votes whose outcome is pending, which the review visits.
    Review,
    /// Nothing; the event's mark can be cleared.
    Idle,
}

/// What an event's ballot box holds for the background tasks.
async fn ballot_box_work(election_event_id: &str) -> Result<BallotBoxWork> {
    let store = get_event_store(election_event_id).await?;
    // This discovery task already runs outside the voting request. Keep each
    // summary refresh bounded, and retain activity until its backlog is empty.
    if store
        .refresh_forensic_summary(election_event_id, SEQUENCER_BATCH)
        .await?
        > 0
    {
        return Ok(BallotBoxWork::Sequencing);
    }
    if store.pending_count(election_event_id).await? > 0 {
        return Ok(BallotBoxWork::Sequencing);
    }
    if store.has_pending_status(election_event_id).await? {
        return Ok(BallotBoxWork::Review);
    }
    Ok(BallotBoxWork::Idle)
}

/// `(tenant_id, election_event_id)` of every event whose ballot box holds ballots
/// waiting for the sequencer. Only events marked when they accepted votes are
/// visited; the mark of an event with nothing left to do is cleared.
#[instrument(err)]
pub async fn events_waiting_for_sequencer() -> Result<Vec<(String, String)>> {
    let databases = get_event_databases().await?;
    let mut waiting = Vec::new();
    for activity in databases.events_with_ballot_activity().await? {
        match ballot_box_work(&activity.election_event_id).await {
            Ok(BallotBoxWork::Sequencing) => {
                waiting.push((activity.tenant_id, activity.election_event_id))
            }
            Ok(BallotBoxWork::Review) => {}
            Ok(BallotBoxWork::Idle) => {
                if let Err(error) = databases.clear_ballot_activity(&activity).await {
                    tracing::error!(
                        "Could not clear the ballot activity of election event {}: {error:?}",
                        activity.election_event_id
                    );
                }
            }
            Err(error) => tracing::error!(
                "Could not read the ballot box of election event {}: {error:?}",
                activity.election_event_id
            ),
        }
    }
    Ok(waiting)
}

/// Events marked when they accepted votes, which may hold votes to review.
pub async fn events_with_ballot_activity() -> Result<Vec<BallotActivity>> {
    get_event_databases()
        .await?
        .events_with_ballot_activity()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use electoral_log::adapters::ballot_box::BallotStatus;

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
        let key = SigningKey::generate()?;
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

        assert_eq!(
            append_pending(&store, &board, &uuid::Uuid::nil().to_string(), &event, &key).await?,
            3
        );
        assert_eq!(store.pending_count(&event).await?, 0);
        // Nothing waits, so a second run appends nothing.
        assert_eq!(
            append_pending(&store, &board, &uuid::Uuid::nil().to_string(), &event, &key).await?,
            0
        );

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
        store.drop_ballot_box(&event).await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "ballot_box_privacy_tests.rs"]
mod privacy_tests;
