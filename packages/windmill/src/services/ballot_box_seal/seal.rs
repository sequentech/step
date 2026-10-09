// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sealing one ballot box (VOTE-FREEZE), in two steps.
//!
//! 1. **Checks**, in a short transaction that doesn't wait for a row
//!    another run holds: the seal is pending and due, every enabled
//!    channel is finished, no Datafix vote of the box is in progress, the
//!    event has a bulletin board and no seal for the box is on it yet. Then
//!    the signing key, and the census outside any lock.
//! 2. **The seal**, in one READ COMMITTED transaction: take the box's lock
//!    without waiting, recheck the seal, read every ballot of the box,
//!    decide how each counts, check every Ballot ID, build and hash the
//!    manifest, sign the `BallotBoxSealed` entry and store it on the row.
//!
//! Each attempt that leaves the seal pending records why on the row
//! (`last_attempt_at`, `waiting_reason`). Only a ballot whose Ballot ID
//! doesn't match its content (or that has no content or no Ballot ID), an
//! event without a bulletin board, or a seal for the box already on the
//! bulletin board fails the seal: the row becomes `failed` (the box stays
//! locked) and [`post_failure`] posts a `BallotBoxSealFailed` ERROR entry.
//! A ballot that can't be read, or a configuration that doesn't parse, is
//! an error the next attempt retries. Publishing is [`super::publish`]'s.

use super::deadline::holding_channel;
use super::publish::{deliver, event_board, genuine_entries, sealed_entries, sha256_hex};
use super::record::{area_name, election_name};
use super::{
    error_category, strict_presentation, with_timeout, Census, SealEnvironment, SealErrorCategory,
};
use crate::postgres::ballot_box_seal::{
    mark_failed_with_names, mark_failure_posted, mark_sealed_with_names, record_attempt, try_lock,
    BallotBoxSeal, BallotBoxSealStatus, SealedFields, SealedNames, TryLocked, WaitingReason,
};
use crate::postgres::cast_vote::count_unresolved_cast_votes;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::cast_votes::CastVoteStatus;
use crate::services::election_event_status::get_election_status;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::EventIdString;
use electoral_log::seal::{
    ballot_hash, build, BallotBoxSealManifest, SealDisposition, SealEntry, DEFAULT_SEAL_CHANNEL,
    SEAL_FORMAT_V1,
};
use sequent_core::ballot::{ContestEncryptionPolicy, HashableBallot, SignedHashableBallot};
use sequent_core::encrypt::{hash_ballot, hash_multi_ballot};
use sequent_core::multi_ballot::{HashableMultiBallot, SignedHashableMultiBallot};
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::types::hasura::core::{Election, ElectionEvent, VotingChannels};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignaturePk;
use tokio_postgres::IsolationLevel;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

/// Why a seal fails when the event's log already has a seal for the box
/// (e.g. after restoring a database older than the seal).
pub const ALREADY_ON_BOARD_REASON: &str =
    "a seal for this ballot box is already on the bulletin board";

/// Why a seal fails when the event has no bulletin board to post it to.
pub const NO_BOARD_REASON: &str = "the election event has no bulletin board";

/// What [`seal_box`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealOutcome {
    /// No such seal.
    NotFound,
    /// The seal isn't pending (another run sealed it, or it failed).
    NotPending(BallotBoxSealStatus),
    /// The grace period hasn't ended.
    NotDue,
    /// An enabled channel of the election isn't finished.
    WaitingForChannel(String),
    /// Another run holds the box (or a cast is in flight); the next tick
    /// retries.
    Busy,
    /// Datafix votes of the box are still in progress; the next tick retries.
    WaitingForVotes(i64),
    Sealed,
    Failed(String),
}

/// Why the ballots of a box can't be sealed now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntriesError {
    /// An incident: the seal fails for good.
    Incident(String),
    /// Something to retry (a ballot that can't be read, a vote still in
    /// progress); the seal stays pending.
    Retry(String),
}

/// One `cast_vote` row of the box, as the seal reads it.
#[derive(Debug, Clone)]
pub struct BoxBallot {
    pub id: Uuid,
    pub voter_id: Option<String>,
    pub status: String,
    pub content: Option<String>,
    pub ballot_id: Option<String>,
    pub channel: String,
    pub created_at: Option<DateTime<Utc>>,
}

/// Seals the ballot box of seal `seal_id` if it is pending and due. An
/// error leaves the seal pending, records it on the row, and is returned.
#[instrument(skip(client, environment), err)]
pub async fn seal_box(
    client: &mut DbClient,
    environment: &dyn SealEnvironment,
    seal_id: &Uuid,
) -> Result<SealOutcome> {
    match seal_box_once(client, environment, seal_id).await {
        Ok(outcome) => Ok(outcome),
        Err(seal_error) => {
            // A category code only (shown on the dashboard); the details go
            // to the logs.
            let category = error_category(&seal_error);
            warn!(%seal_id, %category, "The ballot box seal will be retried: {seal_error:#}");
            let reason = WaitingReason::Error(category.to_string());
            if let Err(record_error) = record(client, seal_id, &reason).await {
                warn!(%seal_id, "Error recording the failed attempt: {record_error:#}");
            }
            Err(seal_error)
        }
    }
}

/// Records an attempt in a transaction of its own.
async fn record(client: &mut DbClient, seal_id: &Uuid, reason: &WaitingReason) -> Result<()> {
    let transaction = client.transaction().await?;
    record_attempt(&transaction, seal_id, reason).await?;
    transaction.commit().await?;
    Ok(())
}

async fn seal_box_once(
    client: &mut DbClient,
    environment: &dyn SealEnvironment,
    seal_id: &Uuid,
) -> Result<SealOutcome> {
    // 1. The checks, then the census outside any lock.
    let (key, census, signing_key, names) = {
        let transaction = client.transaction().await?;
        let seal = match try_lock(&transaction, seal_id).await? {
            TryLocked::Locked(seal) => seal,
            TryLocked::Busy => return Ok(SealOutcome::Busy),
            TryLocked::NotFound => return Ok(SealOutcome::NotFound),
        };
        if seal.status != BallotBoxSealStatus::Pending {
            transaction.commit().await?;
            return Ok(SealOutcome::NotPending(seal.status));
        }
        if seal.grace_deadline > Utc::now() {
            record_attempt(&transaction, seal_id, &WaitingReason::Deadline).await?;
            transaction.commit().await?;
            return Ok(SealOutcome::NotDue);
        }
        let (election_event, election) = event_and_election(&transaction, &seal).await?;
        if let Some(channel) = open_channel(&election).context(SealErrorCategory::Settings)? {
            record_attempt(
                &transaction,
                seal_id,
                &WaitingReason::ChannelOpen(channel.clone()),
            )
            .await?;
            transaction.commit().await?;
            return Ok(SealOutcome::WaitingForChannel(channel));
        }
        if let Some(outcome) = waiting_for_votes(&transaction, &seal).await? {
            transaction.commit().await?;
            return Ok(outcome);
        }
        let names = SealedNames {
            election_name: election_name(&election_event, &election),
            area_name: area_name(
                &transaction,
                &seal.tenant_id.to_string(),
                &seal.area_id.to_string(),
            )
            .await?,
        };
        let Ok(board) = event_board(&election_event) else {
            return fail(transaction, &seal, NO_BOARD_REASON.to_string(), &names).await;
        };
        let signing_key = with_timeout(
            "Reading the event's signing key",
            environment.signing_key(
                &transaction,
                &seal.tenant_id.to_string(),
                &seal.election_event_id.to_string(),
                &board,
            ),
        )
        .await
        .context(SealErrorCategory::Keystore)?;
        // Only a seal signed by the event key counts; anything else on the
        // log is ignored.
        let entries = sealed_entries(
            &board,
            &seal.election_id.to_string(),
            &seal.area_id.to_string(),
        )
        .await
        .context(SealErrorCategory::Board)?;
        if !genuine_entries(entries, &StrandSignaturePk::from_sk(&signing_key)?).is_empty() {
            return fail(
                transaction,
                &seal,
                ALREADY_ON_BOARD_REASON.to_string(),
                &names,
            )
            .await;
        }
        transaction.commit().await?;
        let alias = election
            .external_id
            .clone()
            .filter(|alias| !alias.is_empty())
            .unwrap_or_else(|| election.id.clone());
        let census = environment
            .census(&election_event, &seal.area_id.to_string(), &alias)
            .await
            .context(SealErrorCategory::Census)?;
        (seal.lock_key(), census, signing_key, names)
    };

    // 2. The seal, under the box lock.
    let transaction = client
        .build_transaction()
        .isolation_level(IsolationLevel::ReadCommitted)
        .start()
        .await?;
    let locked: bool = transaction
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
            &[&key],
        )
        .await
        .with_context(|| format!("Error locking ballot box {key}"))?
        .try_get(0)?;
    if !locked {
        // The holder may be a cast in flight: say so on the row.
        record_attempt(&transaction, seal_id, &WaitingReason::Busy).await?;
        transaction.commit().await?;
        return Ok(SealOutcome::Busy);
    }
    let seal = match try_lock(&transaction, seal_id).await? {
        TryLocked::Locked(seal) => seal,
        TryLocked::Busy => return Ok(SealOutcome::Busy),
        TryLocked::NotFound => return Ok(SealOutcome::NotFound),
    };
    if seal.status != BallotBoxSealStatus::Pending {
        transaction.commit().await?;
        return Ok(SealOutcome::NotPending(seal.status));
    }
    if let Some(outcome) = waiting_for_votes(&transaction, &seal).await? {
        transaction.commit().await?;
        return Ok(outcome);
    }

    let (election_event, _) = event_and_election(&transaction, &seal).await?;
    let presentation = strict_presentation(&election_event).context(SealErrorCategory::Settings)?;
    let ballots = read_box(&transaction, &seal).await?;
    let entries = match entries(
        &ballots,
        &census,
        presentation.contest_encryption_policy.unwrap_or_default(),
    ) {
        Ok(entries) => entries,
        Err(EntriesError::Incident(reason)) => {
            return fail(transaction, &seal, reason, &names).await
        }
        Err(EntriesError::Retry(reason)) => {
            return Err(anyhow!(reason).context(SealErrorCategory::Ballots))
        }
    };

    let signing_data = SigningData::new(signing_key.clone(), "", signing_key);
    let SealedNames {
        election_name,
        area_name,
    } = names;

    // Whole seconds, rounded up, so the row and the signed manifest say the
    // same time and it is never before the deadline.
    let now = Utc::now();
    let seconds = now.timestamp() + i64::from(now.timestamp_subsec_nanos() > 0);
    let sealed_at = DateTime::<Utc>::from_timestamp(seconds, 0)
        .ok_or_else(|| anyhow!("The current time is not representable"))?;
    let built = build(BallotBoxSealManifest {
        format: SEAL_FORMAT_V1.to_string(),
        tenant_id: seal.tenant_id.to_string(),
        election_event_id: seal.election_event_id.to_string(),
        election_id: seal.election_id.to_string(),
        area_id: seal.area_id.to_string(),
        closed_at: unix_seconds(seal.closed_at)?,
        grace_deadline: unix_seconds(seal.grace_deadline)?,
        sealed_at: unix_seconds(sealed_at)?,
        close_request_id: seal.close_request_id.map(|id| id.to_string()),
        eligible_voters: census.len() as u64,
        entries,
    })?;
    let message = Message::ballot_box_sealed_message(
        &built.manifest,
        built.hash,
        &election_name,
        &area_name,
        &signing_data,
    )?;
    let fields = SealedFields {
        sealed_at,
        ballots_in_box: i64::try_from(built.manifest.ballots_in_box())?,
        ballots_counted: i64::try_from(built.manifest.ballots_counted())?,
        seal_hash: hex::encode(built.hash),
        manifest: built.bytes,
        signed_message: message.strand_serialize()?,
    };
    mark_sealed_with_names(
        &transaction,
        &seal.id,
        &fields,
        &SealedNames {
            election_name: election_name.clone(),
            area_name: area_name.clone(),
        },
    )
    .await?;
    transaction.commit().await?;
    info!(
        seal_id = %seal.id,
        ballots_in_box = fields.ballots_in_box,
        ballots_counted = fields.ballots_counted,
        seal_hash = %fields.seal_hash,
        "Sealed the ballot box"
    );
    Ok(SealOutcome::Sealed)
}

/// The first enabled channel of the election that isn't finished (see
/// [`super::deadline`]), by name.
fn open_channel(election: &Election) -> Result<Option<String>> {
    let status = get_election_status(election.status.clone()).unwrap_or_default();
    let channels: VotingChannels = election
        .voting_channels
        .clone()
        .map(deserialize_value)
        .transpose()
        .context("Failed to deserialize the election's voting channels")?
        .unwrap_or_default();
    Ok(holding_channel(&status, &channels).map(|channel| channel.to_string()))
}

/// `WaitingForVotes` (recorded on the row) while Datafix votes of the box
/// are in progress.
async fn waiting_for_votes(
    transaction: &Transaction<'_>,
    seal: &BallotBoxSeal,
) -> Result<Option<SealOutcome>> {
    let unresolved = count_unresolved_cast_votes(
        transaction,
        &seal.tenant_id,
        &seal.election_event_id,
        &seal.election_id,
        &seal.area_id,
    )
    .await?;
    if unresolved == 0 {
        return Ok(None);
    }
    // Log once per change of the count, not every tick.
    let reason = WaitingReason::DatafixVotes(unresolved);
    if seal.waiting_reason.as_deref() != Some(reason.to_string().as_str()) {
        warn!(
            seal_id = %seal.id,
            unresolved,
            "Ballot box past its seal deadline waits for Datafix votes in progress"
        );
    }
    record_attempt(transaction, &seal.id, &reason).await?;
    Ok(Some(SealOutcome::WaitingForVotes(unresolved)))
}

async fn event_and_election(
    transaction: &Transaction<'_>,
    seal: &BallotBoxSeal,
) -> Result<(ElectionEvent, Election)> {
    let tenant_id = seal.tenant_id.to_string();
    let election_event_id = seal.election_event_id.to_string();
    let election_id = seal.election_id.to_string();
    let election_event =
        get_election_event_by_id(transaction, &tenant_id, &election_event_id).await?;
    let election = get_election_by_id(transaction, &tenant_id, &election_event_id, &election_id)
        .await?
        .ok_or_else(|| anyhow!("Election {election_id} not found"))?;
    Ok((election_event, election))
}

/// Fails the seal: the row becomes `failed` with `reason`. The caller then
/// posts its `BallotBoxSealFailed` entry with [`post_failure`].
async fn fail(
    transaction: Transaction<'_>,
    seal: &BallotBoxSeal,
    reason: String,
    names: &SealedNames,
) -> Result<SealOutcome> {
    mark_failed_with_names(&transaction, &seal.id, &reason, names).await?;
    transaction.commit().await?;
    error!(seal_id = %seal.id, %reason, "The ballot box could not be sealed: it stays locked");
    Ok(SealOutcome::Failed(reason))
}

/// Posts the `BallotBoxSealFailed` ERROR entry of a failed seal and records
/// it (`failure_posted_at`), so the dispatcher stops repeating it. The
/// delivery id and payload are fixed per seal, so posting it again is a
/// no-op. An event without a bulletin board has nowhere to post it: that
/// is recorded too, with an `error!`. Does nothing unless the seal failed
/// and its entry isn't recorded yet. Returns whether it inserted the entry.
#[instrument(skip(client, environment), err)]
pub async fn post_failure(
    client: &mut DbClient,
    environment: &dyn SealEnvironment,
    seal_id: &Uuid,
) -> Result<bool> {
    let transaction = client.transaction().await?;
    let TryLocked::Locked(seal) = try_lock(&transaction, seal_id).await? else {
        return Ok(false);
    };
    let (BallotBoxSealStatus::Failed, Some(reason), None) = (
        seal.status,
        seal.failure_reason.clone(),
        seal.failure_posted_at,
    ) else {
        return Ok(false);
    };
    let (election_event, election) = event_and_election(&transaction, &seal).await?;
    let Ok(board) = event_board(&election_event) else {
        error!(%seal_id, %reason, "No bulletin board to post the failed seal to");
        mark_failure_posted(&transaction, seal_id).await?;
        transaction.commit().await?;
        return Ok(false);
    };
    let signing_key = environment
        .signing_key(
            &transaction,
            &seal.tenant_id.to_string(),
            &seal.election_event_id.to_string(),
            &board,
        )
        .await?;
    let area_name = area_name(
        &transaction,
        &seal.tenant_id.to_string(),
        &seal.area_id.to_string(),
    )
    .await?;
    let message = Message::ballot_box_seal_failed_message(
        EventIdString(seal.election_event_id.to_string()),
        &seal.election_id.to_string(),
        &seal.area_id.to_string(),
        &election_name(&election_event, &election),
        &area_name,
        reason.clone(),
        &SigningData::new(signing_key.clone(), "", signing_key),
    )?;
    let (delivery_id, payload_hash) = failure_delivery(&seal.id, &reason);
    let inserted = deliver(&board, &delivery_id, &payload_hash, &message).await?;
    mark_failure_posted(&transaction, seal_id).await?;
    transaction.commit().await?;
    if inserted {
        error!(seal_id = %seal.id, %reason, "Posted BallotBoxSealFailed to the electoral log");
    }
    Ok(inserted)
}

/// The fixed delivery id and payload hash of a seal's failure entry.
pub fn failure_delivery(seal_id: &Uuid, reason: &str) -> (String, String) {
    (
        sha256_hex(format!("ballot-box-seal-failed:{seal_id}").as_bytes()),
        sha256_hex(format!("{seal_id}:{reason}").as_bytes()),
    )
}

/// Every `cast_vote` row of the box, in any status.
async fn read_box(transaction: &Transaction<'_>, seal: &BallotBoxSeal) -> Result<Vec<BoxBallot>> {
    let rows = transaction
        .query(
            "SELECT id, voter_id_string, status, content, ballot_id,
                    COALESCE(annotations->>'voting_channel', $5) AS channel, created_at
             FROM sequent_backend.cast_vote
             WHERE tenant_id = $1 AND election_event_id = $2 AND election_id = $3
               AND area_id = $4",
            &[
                &seal.tenant_id,
                &seal.election_event_id,
                &seal.election_id,
                &seal.area_id,
                &DEFAULT_SEAL_CHANNEL,
            ],
        )
        .await
        .context("Error reading the ballots of the box")?;
    rows.into_iter()
        .map(|row| {
            Ok(BoxBallot {
                id: row.try_get("id")?,
                voter_id: row.try_get("voter_id_string")?,
                status: row.try_get("status")?,
                content: row.try_get("content")?,
                ballot_id: row.try_get("ballot_id")?,
                channel: row.try_get("channel")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .collect()
}

/// How each ballot of the box counts, as the tally used to decide:
/// each voter's latest valid ballot (by `created_at`, then id) is counted,
/// with the voter's census weight, if the voter is in the census, and not
/// eligible otherwise; an older valid ballot is replaced; a discarded one
/// is discarded. A ballot without content or Ballot ID, or whose Ballot
/// ID doesn't match its content, is an incident; one that can't be read is
/// retried.
pub fn entries(
    ballots: &[BoxBallot],
    census: &Census,
    contest_encryption_policy: ContestEncryptionPolicy,
) -> std::result::Result<Vec<SealEntry>, EntriesError> {
    let valid = CastVoteStatus::Valid.to_string();
    // Each voter's latest valid ballot.
    let mut latest: HashMap<Option<&str>, &BoxBallot> = HashMap::new();
    for ballot in ballots.iter().filter(|ballot| ballot.status == valid) {
        latest
            .entry(ballot.voter_id.as_deref())
            .and_modify(|current| {
                if (ballot.created_at, ballot.id) > (current.created_at, current.id) {
                    *current = ballot;
                }
            })
            .or_insert(ballot);
    }
    let candidates: HashSet<Uuid> = latest.values().map(|ballot| ballot.id).collect();

    let mut entries = Vec::with_capacity(ballots.len());
    for ballot in ballots {
        let (Some(content), Some(ballot_id)) = (&ballot.content, &ballot.ballot_id) else {
            return Err(EntriesError::Incident(format!(
                "the ballot {} has no content or no Ballot ID",
                ballot.id
            )));
        };
        let computed =
            computed_ballot_id(content, &contest_encryption_policy).map_err(|error| {
                EntriesError::Retry(format!("the ballot {} can't be read: {error}", ballot.id))
            })?;
        if computed != *ballot_id {
            return Err(EntriesError::Incident(format!(
                "a ballot does not match its Ballot ID (stored {ballot_id}, content hashes to {computed})"
            )));
        }
        let status = CastVoteStatus::from_str(&ballot.status).map_err(|_| {
            EntriesError::Retry(format!(
                "the ballot {} has status {}",
                ballot.id, ballot.status
            ))
        })?;
        let (disposition, weight) = match status {
            CastVoteStatus::Discarded => (SealDisposition::Discarded, 0),
            CastVoteStatus::InProgress => {
                return Err(EntriesError::Retry(format!(
                    "the ballot {} is still in progress",
                    ballot.id
                )))
            }
            CastVoteStatus::Valid if !candidates.contains(&ballot.id) => {
                (SealDisposition::Replaced, 0)
            }
            CastVoteStatus::Valid => match ballot
                .voter_id
                .as_ref()
                .and_then(|voter_id| census.get(voter_id))
            {
                Some(weight) => (SealDisposition::Counted, *weight),
                None => (SealDisposition::NotEligible, 0),
            },
        };
        entries.push(SealEntry {
            ballot_hash: ballot_hash(content)
                .map_err(|error| EntriesError::Retry(error.to_string()))?,
            ballot_id: ballot_id.clone(),
            disposition,
            weight,
            channel: ballot.channel.clone(),
        });
    }
    Ok(entries)
}

/// The Ballot ID of a stored ballot's content, as the cast computes it.
pub fn computed_ballot_id(content: &str, policy: &ContestEncryptionPolicy) -> Result<String> {
    Ok(match policy {
        ContestEncryptionPolicy::MULTIPLE_CONTESTS => {
            let signed: SignedHashableMultiBallot =
                deserialize_str(content).map_err(|error| anyhow!("{error}"))?;
            let hashable: HashableMultiBallot = (&signed).try_into()?;
            hash_multi_ballot(&hashable)?
        }
        ContestEncryptionPolicy::SINGLE_CONTEST => {
            let signed: SignedHashableBallot =
                deserialize_str(content).map_err(|error| anyhow!("{error}"))?;
            let hashable: HashableBallot = (&signed).try_into()?;
            hash_ballot(&hashable)?
        }
    })
}

fn unix_seconds(time: DateTime<Utc>) -> Result<u64> {
    Ok(u64::try_from(time.timestamp())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ballot_box_seal::census_from_csv;
    use crate::services::join::{merge_join_csv, MultiplicitySource};
    use crate::services::users::VoterMultiplicityColumn;
    use sequent_core::ballot::{SignedHashableBallot, TYPES_VERSION};
    use std::io::Write;

    /// A stored ballot's content (no contests, which hashing allows) and
    /// its Ballot ID.
    fn ballot(seed: &str) -> (String, String) {
        let content = serde_json::to_string(&SignedHashableBallot {
            version: TYPES_VERSION,
            issue_date: seed.to_string(),
            contests: vec![],
            config: seed.to_string(),
            ballot_style_hash: seed.to_string(),
            voter_signing_pk: None,
            voter_ballot_signature: None,
        })
        .unwrap();
        let ballot_id =
            computed_ballot_id(&content, &ContestEncryptionPolicy::SINGLE_CONTEST).unwrap();
        (content, ballot_id)
    }

    fn at(minute: i64) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(1_800_000_000 + minute * 60, 0)
    }

    fn row(voter: Option<&str>, status: &str, seed: &str, minute: i64) -> BoxBallot {
        let (content, ballot_id) = ballot(seed);
        BoxBallot {
            id: Uuid::new_v4(),
            voter_id: voter.map(str::to_owned),
            status: status.to_string(),
            content: Some(content),
            ballot_id: Some(ballot_id),
            channel: DEFAULT_SEAL_CHANNEL.to_string(),
            created_at: at(minute),
        }
    }

    fn disposition_of(entries: &[SealEntry], ballot: &BoxBallot) -> (SealDisposition, u64) {
        let entry = entries
            .iter()
            .find(|entry| Some(&entry.ballot_id) == ballot.ballot_id.as_ref())
            .unwrap();
        (entry.disposition, entry.weight)
    }

    #[test]
    fn each_ballot_gets_its_disposition() {
        let census = Census::from([("alice".to_string(), 1), ("bob".to_string(), 3)]);
        let alice_old = row(Some("alice"), "valid", "a1", 1);
        let alice_new = row(Some("alice"), "valid", "a2", 2);
        let bob = row(Some("bob"), "valid", "b1", 1);
        let carol = row(Some("carol"), "valid", "c1", 1);
        let dave = row(Some("dave"), "discarded", "d1", 1);
        let ballots = vec![
            alice_old.clone(),
            alice_new.clone(),
            bob.clone(),
            carol.clone(),
            dave.clone(),
        ];
        let entries = entries(&ballots, &census, ContestEncryptionPolicy::SINGLE_CONTEST).unwrap();
        assert_eq!(entries.len(), 5);
        assert_eq!(
            disposition_of(&entries, &alice_old),
            (SealDisposition::Replaced, 0)
        );
        assert_eq!(
            disposition_of(&entries, &alice_new),
            (SealDisposition::Counted, 1)
        );
        assert_eq!(
            disposition_of(&entries, &bob),
            (SealDisposition::Counted, 3)
        );
        assert_eq!(
            disposition_of(&entries, &carol),
            (SealDisposition::NotEligible, 0)
        );
        assert_eq!(
            disposition_of(&entries, &dave),
            (SealDisposition::Discarded, 0)
        );
        // The entry hashes the stored content exactly.
        let entry = entries
            .iter()
            .find(|entry| Some(&entry.ballot_id) == bob.ballot_id.as_ref())
            .unwrap();
        assert_eq!(
            entry.ballot_hash,
            ballot_hash(bob.content.as_deref().unwrap()).unwrap()
        );
    }

    #[test]
    fn the_latest_ballot_is_by_time_then_id_and_a_missing_time_is_oldest() {
        let census = Census::from([("alice".to_string(), 1)]);
        let untimed = BoxBallot {
            created_at: None,
            ..row(Some("alice"), "valid", "u", 0)
        };
        let mut first = row(Some("alice"), "valid", "x", 5);
        let mut second = row(Some("alice"), "valid", "y", 5);
        // Same time: the larger id is the latest.
        first.id = Uuid::from_u128(1);
        second.id = Uuid::from_u128(2);
        let ballots = vec![untimed.clone(), second.clone(), first.clone()];
        let entries = entries(&ballots, &census, ContestEncryptionPolicy::SINGLE_CONTEST).unwrap();
        assert_eq!(
            disposition_of(&entries, &second).0,
            SealDisposition::Counted
        );
        assert_eq!(
            disposition_of(&entries, &first).0,
            SealDisposition::Replaced
        );
        assert_eq!(
            disposition_of(&entries, &untimed).0,
            SealDisposition::Replaced
        );
    }

    #[test]
    fn ballots_without_a_voter_are_one_voter_and_not_eligible() {
        let census = Census::new();
        let older = row(None, "valid", "n1", 1);
        let newer = row(None, "valid", "n2", 2);
        let entries = entries(
            &[older.clone(), newer.clone()],
            &census,
            ContestEncryptionPolicy::SINGLE_CONTEST,
        )
        .unwrap();
        assert_eq!(
            disposition_of(&entries, &older).0,
            SealDisposition::Replaced
        );
        assert_eq!(
            disposition_of(&entries, &newer).0,
            SealDisposition::NotEligible
        );
    }

    #[test]
    fn a_ballot_id_mismatch_fails_the_seal() {
        let mut tampered = row(Some("alice"), "valid", "a1", 1);
        tampered.ballot_id = Some(ballot("other").1);
        let reason = entries(
            &[tampered],
            &Census::new(),
            ContestEncryptionPolicy::SINGLE_CONTEST,
        )
        .unwrap_err();
        assert!(
            matches!(&reason, EntriesError::Incident(text) if text.starts_with("a ballot does not match its Ballot ID")),
            "{reason:?}"
        );
    }

    #[test]
    fn a_ballot_that_cant_be_read_is_retried_not_an_incident() {
        let mut unreadable = row(Some("alice"), "valid", "a1", 1);
        // Another types version: the content doesn't convert.
        unreadable.content = Some(
            unreadable
                .content
                .unwrap()
                .replace(&format!("\"version\":{TYPES_VERSION}"), "\"version\":999"),
        );
        let reason = entries(
            &[unreadable],
            &Census::new(),
            ContestEncryptionPolicy::SINGLE_CONTEST,
        )
        .unwrap_err();
        assert!(
            matches!(&reason, EntriesError::Retry(text) if text.contains("can't be read")),
            "{reason:?}"
        );
    }

    #[test]
    fn a_ballot_without_content_or_id_fails_the_seal() {
        for missing_content in [true, false] {
            let mut ballot = row(Some("alice"), "valid", "a1", 1);
            if missing_content {
                ballot.content = None;
            } else {
                ballot.ballot_id = None;
            }
            let reason = entries(
                &[ballot.clone()],
                &Census::new(),
                ContestEncryptionPolicy::SINGLE_CONTEST,
            )
            .unwrap_err();
            assert_eq!(
                reason,
                EntriesError::Incident(format!(
                    "the ballot {} has no content or no Ballot ID",
                    ballot.id
                ))
            );
        }
    }

    #[test]
    fn an_empty_box_has_no_entries() {
        assert!(
            entries(&[], &Census::new(), ContestEncryptionPolicy::SINGLE_CONTEST)
                .unwrap()
                .is_empty()
        );
    }

    fn file_with(text: &str) -> std::fs::File {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(text.as_bytes()).unwrap();
        use std::io::Seek;
        file.rewind().unwrap();
        file
    }

    /// The weights and census the seal records are the multiplicities and
    /// eligible voters the tally's `merge_join_csv` computes from the same
    /// voter dump.
    #[test]
    fn counted_weights_match_merge_join_csv() {
        let cases = [
            (VoterMultiplicityColumn::None, None, "alice\nbob\ncarol\n"),
            (
                VoterMultiplicityColumn::DelegateCount,
                Some(MultiplicitySource::DelegateCount(1)),
                "alice,2\nbob,0\ncarol,5\n",
            ),
            (
                VoterMultiplicityColumn::VoteWeight,
                Some(MultiplicitySource::VoteWeight(1)),
                "alice,7\nbob,1\ncarol,4\n",
            ),
        ];
        for (column, source, voters) in cases {
            let census = census_from_csv(voters.as_bytes(), column).unwrap();
            let alice = row(Some("alice"), "valid", "a", 1);
            let bob = row(Some("bob"), "valid", "b", 1);
            let dave = row(Some("dave"), "valid", "d", 1);
            let ballots = [alice.clone(), bob.clone(), dave.clone()];
            let sealed =
                entries(&ballots, &census, ContestEncryptionPolicy::SINGLE_CONTEST).unwrap();

            // The tally's join over the same voters and the voters' latest
            // valid ballots, both sorted by voter id.
            let ballots_csv: String = [&alice, &bob, &dave]
                .iter()
                .map(|ballot| {
                    format!(
                        "{},{},{}\n",
                        ballot.voter_id.as_deref().unwrap(),
                        ballot.ballot_id.as_deref().unwrap(),
                        ballot.channel
                    )
                })
                .collect();
            let joined = merge_join_csv(
                &file_with(&ballots_csv),
                &file_with(voters),
                0,
                0,
                1,
                Some(2),
                source,
            )
            .unwrap();
            assert_eq!(census.len() as u64, joined.eligible_voters, "{column:?}");
            let tally: HashMap<String, u64> = joined.ballot_contents.into_iter().collect();
            for ballot in [&alice, &bob] {
                let (disposition, weight) = disposition_of(&sealed, ballot);
                assert_eq!(disposition, SealDisposition::Counted);
                assert_eq!(
                    Some(&weight),
                    tally.get(ballot.ballot_id.as_deref().unwrap()),
                    "{column:?}"
                );
            }
            assert_eq!(
                disposition_of(&sealed, &dave).0,
                SealDisposition::NotEligible
            );
            assert_eq!(joined.ballots_without_voter, 1, "{column:?}");
        }
    }

    #[test]
    fn a_vote_weight_out_of_range_is_refused_like_the_tally() {
        assert!(
            census_from_csv("alice,0\n".as_bytes(), VoterMultiplicityColumn::VoteWeight).is_err()
        );
    }
}
