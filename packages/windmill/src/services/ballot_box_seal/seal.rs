// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sealing one ballot box (VOTE-FREEZE), in two steps.
//!
//! 1. **Checks**, in a short Hasura transaction that doesn't wait for a row
//!    another run holds: the seal is pending and due, no channel holds the
//!    seal, the event has a bulletin board, its electoral-log database holds
//!    no seal of the box yet (if it does, the row takes it: an earlier run
//!    stored the seal and stopped before updating the row), and no Datafix
//!    vote of the box is in progress. Then the signing key, the census
//!    outside any lock, and the sequencer: every accepted ballot of the box
//!    must be on the electoral log before the seal.
//! 2. **The seal**, in one READ COMMITTED transaction of the event's
//!    electoral-log database: take the box's seal lock without waiting,
//!    recheck the seal and the sequencer, read every ballot of the box,
//!    check that the channel of each is closed, decide how each counts,
//!    check every Ballot ID, build and hash the manifest, sign the
//!    `BallotBoxSealed` record, append it to the board and store the seal.
//!    From the commit on, the database's guard refuses every write to the
//!    box. Then the row of `sequent_backend.ballot_box_seal` takes the seal.
//!
//! Each attempt that leaves the seal pending records why on the row
//! (`last_attempt_at`, `waiting_reason`), e.g. a channel that holds the
//! seal ([`super::deadline::holding_channel`]), a ballot of a channel that
//! isn't closed, or ballots the sequencer hasn't appended yet. Only a ballot
//! whose Ballot ID doesn't match its content (or that has no content or no
//! Ballot ID, or whose channel is none of the voting channels), a ballot the
//! sequencer removed from its queue without its record on the board, an
//! event without a bulletin board, or a seal record of the box already on
//! the bulletin board fails the seal: the seal is stored `failed` with its
//! `BallotBoxSealFailed` ERROR record, in the same transaction, and the box
//! stays locked. A ballot that can't be read, or a configuration that
//! doesn't parse, is an error the next attempt retries. Publishing is
//! [`super::publish`]'s.

use super::deadline::{holding_channel, HoldingChannel};
use super::publish::event_board;
use super::record::{area_name, election_name};
use super::{error_category, with_timeout, Census, SealEnvironment, SealErrorCategory};
use crate::postgres::ballot_box_seal::{
    mark_failed_with_names, mark_sealed_with_names, record_attempt, try_lock, BallotBoxSeal,
    BallotBoxSealStatus, SealLogRecord, SealedFields, SealedNames, TryLocked, WaitingReason,
};
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::ballot_box::{wait_for_sequencer, BallotFormat};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::adapters::ballot_box::BallotStatus;
use electoral_log::adapters::ballot_box_seal::{
    ballot_box_seal_in, box_ballots, insert_seal, try_lock_ballot_box, BallotBoxId, BoxBallot,
    NewSeal, SealOutcome as StoredOutcome, StoredSeal,
};
use electoral_log::adapters::postgres::{append_in, PostgresStore};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::EventIdString;
use electoral_log::seal::{
    ballot_hash, build, BallotBoxSealManifest, SealDisposition, SealEntry, SEAL_FORMAT_V1,
};
use electoral_log::LogEntry;
use sequent_core::ballot::{
    ElectionStatus, HashableBallot, SignedHashableBallot, VotingStatus, VotingStatusChannel,
};
use sequent_core::encrypt::{hash_ballot, hash_multi_ballot};
use sequent_core::multi_ballot::{HashableMultiBallot, SignedHashableMultiBallot};
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::types::hasura::core::{Election, ElectionEvent, VotingChannels};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;
use tokio_postgres::IsolationLevel;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;
use voting_crypto::signing::SigningKey;
use voting_crypto::RecordSerialize;

/// Why a seal fails when the event's log already has a seal record for the
/// box but its database has no seal.
pub const ALREADY_ON_BOARD_REASON: &str =
    "a seal for this ballot box is already on the bulletin board";

/// Why a seal fails when the event has no bulletin board to post it to.
pub const NO_BOARD_REASON: &str = "the election event has no bulletin board";

/// How long one attempt waits for the sequencer to append the box's
/// accepted ballots before it leaves the seal to the next attempt.
pub const SEAL_SEQUENCER_WAIT: Duration = Duration::from_secs(60);

/// What [`seal_box`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealOutcome {
    /// No such seal.
    NotFound,
    /// The seal isn't pending (another run sealed it, or it failed).
    NotPending(BallotBoxSealStatus),
    /// The grace period hasn't ended.
    NotDue,
    /// A channel of the election holds the seal, or the box has ballots of
    /// a channel that isn't closed.
    WaitingForChannel(String),
    /// Another run holds the box (or a write is in flight); the next tick
    /// retries.
    Busy,
    /// Datafix votes of the box are still in progress; the next tick retries.
    WaitingForVotes(i64),
    /// Accepted ballots of the box are not on the electoral log yet; the
    /// next tick retries.
    WaitingForSequencer(i64),
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

/// The delivery ID of a ballot box's `BallotBoxSealed` record: one per box
/// on the board.
pub fn seal_delivery_id(ballot_box: BallotBoxId<'_>) -> String {
    format!(
        "ballot-box-seal:{}:{}:{}",
        ballot_box.election_event_id, ballot_box.election_id, ballot_box.area_id
    )
}

/// The delivery ID of a ballot box's `BallotBoxSealFailed` record.
pub fn failure_delivery_id(ballot_box: BallotBoxId<'_>) -> String {
    format!(
        "ballot-box-seal-failed:{}:{}:{}",
        ballot_box.election_event_id, ballot_box.election_id, ballot_box.area_id
    )
}

/// The ids of a seal row's ballot box, as the event's database names it.
struct BoxIds {
    election_event_id: String,
    election_id: String,
    area_id: String,
}

impl BoxIds {
    fn of(seal: &BallotBoxSeal) -> Self {
        BoxIds {
            election_event_id: seal.election_event_id.to_string(),
            election_id: seal.election_id.to_string(),
            area_id: seal.area_id.to_string(),
        }
    }

    fn ballot_box(&self) -> BallotBoxId<'_> {
        BallotBoxId {
            election_event_id: &self.election_event_id,
            election_id: &self.election_id,
            area_id: &self.area_id,
        }
    }
}

/// What the checks found to seal a box with.
struct Prepared {
    seal: BallotBoxSeal,
    ids: BoxIds,
    store: PostgresStore,
    board: String,
    status: ElectionStatus,
    signing_key: SigningKey,
    census: Census,
    names: SealedNames,
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
    let prepared = match prepare(client, environment, seal_id).await? {
        Ok(prepared) => prepared,
        Err(outcome) => return Ok(outcome),
    };
    if let Some(outcome) = wait_for_ballots(client, environment, &prepared).await? {
        return Ok(outcome);
    }
    seal_prepared(client, prepared).await
}

/// Step 1: the checks, then the census outside any lock. `Err` is what the
/// attempt ends with.
async fn prepare(
    client: &mut DbClient,
    environment: &dyn SealEnvironment,
    seal_id: &Uuid,
) -> Result<std::result::Result<Prepared, SealOutcome>> {
    let transaction = client.transaction().await?;
    let seal = match try_lock(&transaction, seal_id).await? {
        TryLocked::Locked(seal) => seal,
        TryLocked::Busy => return Ok(Err(SealOutcome::Busy)),
        TryLocked::NotFound => return Ok(Err(SealOutcome::NotFound)),
    };
    if seal.status != BallotBoxSealStatus::Pending {
        transaction.commit().await?;
        return Ok(Err(SealOutcome::NotPending(seal.status)));
    }
    if seal.grace_deadline > Utc::now() {
        record_attempt(&transaction, seal_id, &WaitingReason::Deadline).await?;
        transaction.commit().await?;
        return Ok(Err(SealOutcome::NotDue));
    }
    let (election_event, election) = event_and_election(&transaction, &seal).await?;
    if let Some(holding) = open_channel(&election).context(SealErrorCategory::Settings)? {
        let channel = holding.channel().to_string();
        let reason = match holding {
            HoldingChannel::Enabled(_) => WaitingReason::ChannelOpen(channel.clone()),
            HoldingChannel::NotEnabled(_) => WaitingReason::ChannelNotEnabled(channel.clone()),
        };
        record_attempt(&transaction, seal_id, &reason).await?;
        transaction.commit().await?;
        return Ok(Err(SealOutcome::WaitingForChannel(channel)));
    }
    let status = election_status(election.status.clone()).context(SealErrorCategory::Settings)?;
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
        // Without a board the event takes no ballots and has nowhere to
        // record the seal: only the row fails.
        mark_failed_with_names(&transaction, &seal.id, NO_BOARD_REASON, None, &names).await?;
        transaction.commit().await?;
        error!(seal_id = %seal.id, reason = NO_BOARD_REASON, "The ballot box could not be sealed");
        return Ok(Err(SealOutcome::Failed(NO_BOARD_REASON.to_string())));
    };
    let ids = BoxIds::of(&seal);
    let store = environment
        .event_store(&ids.election_event_id)
        .await
        .context(SealErrorCategory::Board)?;
    if let Some(stored) = store
        .ballot_box_seal(ids.ballot_box())
        .await
        .context(SealErrorCategory::Board)?
    {
        let outcome = take_stored_seal(&transaction, &seal, &stored, &names).await?;
        transaction.commit().await?;
        return Ok(Err(outcome));
    }
    if let Some(outcome) = waiting_for_votes(&transaction, &seal, &store, &ids).await? {
        transaction.commit().await?;
        return Ok(Err(outcome));
    }
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
    Ok(Ok(Prepared {
        seal,
        ids,
        store,
        board,
        status,
        signing_key,
        census,
        names,
    }))
}

/// Asks the sequencer for the box's accepted ballots that aren't on the
/// electoral log yet and waits for them, at most the environment's
/// [`SealEnvironment::sequencer_wait`].
/// `Some` when some still wait (recorded on the row).
async fn wait_for_ballots(
    client: &mut DbClient,
    environment: &dyn SealEnvironment,
    prepared: &Prepared,
) -> Result<Option<SealOutcome>> {
    let ids = &prepared.ids;
    let unsequenced = || {
        prepared
            .store
            .unsequenced_count(&ids.election_event_id, &ids.election_id, &ids.area_id)
    };
    if unsequenced().await.context(SealErrorCategory::Board)? == 0 {
        return Ok(None);
    }
    environment
        .request_sequencer(&ids.election_event_id)
        .await
        .context(SealErrorCategory::Board)?;
    let waiting = wait_for_sequencer(environment.sequencer_wait(), unsequenced)
        .await
        .context(SealErrorCategory::Board)?;
    if waiting == 0 {
        return Ok(None);
    }
    record(
        client,
        &prepared.seal.id,
        &WaitingReason::Sequencer(waiting),
    )
    .await?;
    Ok(Some(SealOutcome::WaitingForSequencer(waiting)))
}

/// Step 2: the seal, under the box's seal lock in the event's database.
async fn seal_prepared(client: &mut DbClient, prepared: Prepared) -> Result<SealOutcome> {
    let Prepared {
        seal,
        ids,
        store,
        board,
        status,
        signing_key,
        census,
        names,
    } = prepared;
    let ballot_box = ids.ballot_box();
    let mut event_client = store.client().await.context(SealErrorCategory::Board)?;
    let tx = event_client
        .build_transaction()
        .isolation_level(IsolationLevel::ReadCommitted)
        .start()
        .await
        .context(SealErrorCategory::Board)?;
    if !try_lock_ballot_box(&tx, ballot_box).await? {
        // The holder may be a write in flight: say so on the row.
        drop(tx);
        record(client, &seal.id, &WaitingReason::Busy).await?;
        return Ok(SealOutcome::Busy);
    }
    if let Some(stored) = ballot_box_seal_in(&tx, ballot_box).await? {
        drop(tx);
        return take_stored(client, &seal, &stored, &names).await;
    }
    let unsequenced = store
        .unsequenced_count(&ids.election_event_id, &ids.election_id, &ids.area_id)
        .await?;
    if unsequenced > 0 {
        drop(tx);
        record(client, &seal.id, &WaitingReason::Sequencer(unsequenced)).await?;
        return Ok(SealOutcome::WaitingForSequencer(unsequenced));
    }
    let unresolved = store
        .pending_status_count(&ids.election_event_id, &ids.election_id, &ids.area_id)
        .await?;
    if unresolved > 0 {
        drop(tx);
        record(client, &seal.id, &WaitingReason::DatafixVotes(unresolved)).await?;
        return Ok(SealOutcome::WaitingForVotes(unresolved));
    }
    let signing_data = SigningData::new(signing_key.clone(), "", signing_key);
    let unrecorded = store
        .unrecorded_count(
            &board,
            &ids.election_event_id,
            &ids.election_id,
            &ids.area_id,
        )
        .await?;
    if unrecorded > 0 {
        let reason = unrecorded_reason(unrecorded);
        return fail(
            client,
            tx,
            &seal,
            &board,
            &ids,
            reason,
            &names,
            &signing_data,
        )
        .await;
    }

    let mut ballots = Vec::new();
    box_ballots(&tx, ballot_box, |ballot| {
        ballots.push(ballot);
        Ok(())
    })
    .await
    .context(SealErrorCategory::Ballots)?;
    // The ballots are evidence: none may come from a channel that isn't
    // closed, whatever the election enables.
    match open_ballot_channel(&ballots, &status) {
        Ok(None) => {}
        Ok(Some(channel)) => {
            drop(tx);
            let channel = channel.to_string();
            record(
                client,
                &seal.id,
                &WaitingReason::ChannelHasBallots(channel.clone()),
            )
            .await?;
            return Ok(SealOutcome::WaitingForChannel(channel));
        }
        Err(EntriesError::Incident(reason)) => {
            return fail(
                client,
                tx,
                &seal,
                &board,
                &ids,
                reason,
                &names,
                &signing_data,
            )
            .await
        }
        Err(EntriesError::Retry(reason)) => {
            return Err(anyhow!(reason).context(SealErrorCategory::Ballots))
        }
    }
    let entries = match entries(&ballots, &census) {
        Ok(entries) => entries,
        Err(EntriesError::Incident(reason)) => {
            return fail(
                client,
                tx,
                &seal,
                &board,
                &ids,
                reason,
                &names,
                &signing_data,
            )
            .await
        }
        Err(EntriesError::Retry(reason)) => {
            return Err(anyhow!(reason).context(SealErrorCategory::Ballots))
        }
    };

    // Whole seconds, rounded up, so the row and the signed manifest say the
    // same time and it is never before the deadline.
    let now = Utc::now();
    let seconds = now.timestamp() + i64::from(now.timestamp_subsec_nanos() > 0);
    let sealed_at = DateTime::<Utc>::from_timestamp(seconds, 0)
        .ok_or_else(|| anyhow!("The current time is not representable"))?;
    let built = build(BallotBoxSealManifest {
        format: SEAL_FORMAT_V1.to_string(),
        tenant_id: seal.tenant_id.to_string(),
        election_event_id: ids.election_event_id.clone(),
        election_id: ids.election_id.clone(),
        area_id: ids.area_id.clone(),
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
        &names.election_name,
        &names.area_name,
        &signing_data,
    )?;
    let signed_message = message.record_serialize()?;
    let entry = LogEntry {
        delivery_id: seal_delivery_id(ballot_box),
        message: (&message).try_into()?,
    };
    let stored = append_in(&tx, &board, &[entry])
        .await
        .context(SealErrorCategory::Board)?;
    let Some(record) = stored.first() else {
        // The box has no seal, yet its seal record is on the board.
        return fail(
            client,
            tx,
            &seal,
            &board,
            &ids,
            ALREADY_ON_BOARD_REASON.to_string(),
            &names,
            &signing_data,
        )
        .await;
    };
    let outcome = StoredOutcome::Sealed {
        seal_hash: built.hash.to_vec(),
        manifest: built.bytes.clone(),
        ballots_in_box: i64::try_from(built.manifest.ballots_in_box())?,
        ballots_counted: i64::try_from(built.manifest.ballots_counted())?,
    };
    if !insert_seal(
        &tx,
        &NewSeal {
            ballot_box,
            outcome: &outcome,
            signed_message: &signed_message,
            record,
        },
    )
    .await?
    {
        return Err(anyhow!(
            "Ballot box of election {} area {} got a seal while it was locked",
            ids.election_id,
            ids.area_id
        ));
    }
    let log_record = SealLogRecord::from(record);
    tx.commit().await.context(SealErrorCategory::Board)?;
    let fields = SealedFields {
        sealed_at,
        ballots_in_box: i64::try_from(built.manifest.ballots_in_box())?,
        ballots_counted: i64::try_from(built.manifest.ballots_counted())?,
        seal_hash: hex::encode(built.hash),
        manifest: built.bytes,
        signed_message,
        log_record,
    };
    info!(
        seal_id = %seal.id,
        ballots_in_box = fields.ballots_in_box,
        ballots_counted = fields.ballots_counted,
        seal_hash = %fields.seal_hash,
        log_delivery_id = %fields.log_record.delivery_id,
        log_leaf_index = fields.log_record.leaf_index,
        "Sealed the ballot box"
    );
    if let Some(transaction) = pending_row(client, &seal.id).await? {
        mark_sealed_with_names(&transaction, &seal.id, &fields, &names).await?;
        transaction.commit().await?;
    }
    Ok(SealOutcome::Sealed)
}

/// Why a seal fails when ballots of the box have no cast-vote record on the
/// board although the sequencer is done with them.
pub fn unrecorded_reason(unrecorded: i64) -> String {
    format!(
        "{unrecorded} ballot(s) of the ballot box have no cast-vote record on the bulletin board"
    )
}

/// A transaction holding the seal row `seal_id` while it is pending, to
/// update it once the event's database holds the seal; `None` when another
/// run holds the row or already updated it.
async fn pending_row<'a>(
    client: &'a mut DbClient,
    seal_id: &Uuid,
) -> Result<Option<Transaction<'a>>> {
    let transaction = client.transaction().await?;
    Ok(match try_lock(&transaction, seal_id).await? {
        TryLocked::Locked(seal) if seal.status == BallotBoxSealStatus::Pending => Some(transaction),
        _ => None,
    })
}

/// The row takes the seal the event's database holds for the box, in
/// `transaction` (which holds the row): an earlier run stored it and
/// stopped before updating the row.
async fn take_stored_seal(
    transaction: &Transaction<'_>,
    seal: &BallotBoxSeal,
    stored: &StoredSeal,
    names: &SealedNames,
) -> Result<SealOutcome> {
    let log_record = SealLogRecord {
        delivery_id: stored.log_delivery_id.clone(),
        log_uid: stored.log_uid,
        leaf_index: stored.log_leaf_index,
    };
    match &stored.outcome {
        StoredOutcome::Sealed {
            seal_hash,
            manifest,
            ballots_in_box,
            ballots_counted,
        } => {
            let sealed_at = BallotBoxSealManifest::from_bytes(manifest)
                .context("The stored seal's manifest does not decode")?
                .sealed_at;
            let sealed_at = DateTime::<Utc>::from_timestamp(i64::try_from(sealed_at)?, 0)
                .ok_or_else(|| anyhow!("The stored seal's time is out of range"))?;
            let fields = SealedFields {
                sealed_at,
                ballots_in_box: *ballots_in_box,
                ballots_counted: *ballots_counted,
                seal_hash: hex::encode(seal_hash),
                manifest: manifest.clone(),
                signed_message: stored.signed_message.clone(),
                log_record,
            };
            mark_sealed_with_names(transaction, &seal.id, &fields, names).await?;
            info!(seal_id = %seal.id, "The seal row took the ballot box's stored seal");
            Ok(SealOutcome::Sealed)
        }
        StoredOutcome::Failed { reason } => {
            mark_failed_with_names(transaction, &seal.id, reason, Some(&log_record), names).await?;
            error!(seal_id = %seal.id, %reason, "The ballot box could not be sealed: it stays locked");
            Ok(SealOutcome::Failed(reason.clone()))
        }
    }
}

/// [`take_stored_seal`] in a transaction of its own.
async fn take_stored(
    client: &mut DbClient,
    seal: &BallotBoxSeal,
    stored: &StoredSeal,
    names: &SealedNames,
) -> Result<SealOutcome> {
    let transaction = client.transaction().await?;
    let outcome = match try_lock(&transaction, &seal.id).await? {
        TryLocked::Locked(row) if row.status == BallotBoxSealStatus::Pending => {
            take_stored_seal(&transaction, &row, stored, names).await?
        }
        TryLocked::Locked(row) => SealOutcome::NotPending(row.status),
        TryLocked::Busy => SealOutcome::Busy,
        TryLocked::NotFound => SealOutcome::NotFound,
    };
    transaction.commit().await?;
    Ok(outcome)
}

/// The first channel of the election that holds its seal (see
/// [`super::deadline`]).
fn open_channel(election: &Election) -> Result<Option<HoldingChannel>> {
    let status = election_status(election.status.clone())?;
    let channels: VotingChannels = election
        .voting_channels
        .clone()
        .map(deserialize_value)
        .transpose()
        .context("Failed to deserialize the election's voting channels")?
        .unwrap_or_default();
    Ok(holding_channel(&status, &channels))
}

/// The election's voting status: unset is every channel NOT_STARTED, and a
/// status that doesn't parse is an error (the seal retries, as for any other
/// setting it can't read), never a guess (R10 S4).
pub(crate) fn election_status(status: Option<serde_json::Value>) -> Result<ElectionStatus> {
    Ok(status
        .map(deserialize_value::<ElectionStatus>)
        .transpose()
        .context("Failed to deserialize the election's voting status")?
        .unwrap_or_default())
}

/// `WaitingForVotes` (recorded on the row) while Datafix votes of the box
/// are in progress.
async fn waiting_for_votes(
    transaction: &Transaction<'_>,
    seal: &BallotBoxSeal,
    store: &PostgresStore,
    ids: &BoxIds,
) -> Result<Option<SealOutcome>> {
    let unresolved = store
        .pending_status_count(&ids.election_event_id, &ids.election_id, &ids.area_id)
        .await
        .context(SealErrorCategory::Board)?;
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

/// Fails the seal in `tx`, which holds the box's seal lock: appends its
/// `BallotBoxSealFailed` ERROR record and stores the failed seal, which
/// keeps the box locked, then the row takes it.
#[allow(clippy::too_many_arguments)]
async fn fail(
    client: &mut DbClient,
    tx: deadpool_postgres::Transaction<'_>,
    seal: &BallotBoxSeal,
    board: &str,
    ids: &BoxIds,
    reason: String,
    names: &SealedNames,
    signing_data: &SigningData,
) -> Result<SealOutcome> {
    let ballot_box = ids.ballot_box();
    let message = Message::ballot_box_seal_failed_message(
        EventIdString(ids.election_event_id.clone()),
        &ids.election_id,
        &ids.area_id,
        &names.election_name,
        &names.area_name,
        reason.clone(),
        signing_data,
    )?;
    let signed_message = message.record_serialize()?;
    let entry = LogEntry {
        delivery_id: failure_delivery_id(ballot_box),
        message: (&message).try_into()?,
    };
    let stored = append_in(&tx, board, &[entry])
        .await
        .context(SealErrorCategory::Board)?;
    let record = stored.first().ok_or_else(|| {
        anyhow!(
            "The failure record of the ballot box of election {} area {} is already on the \
             bulletin board, but the box has no seal",
            ids.election_id,
            ids.area_id
        )
        .context(SealErrorCategory::Board)
    })?;
    let outcome = StoredOutcome::Failed {
        reason: reason.clone(),
    };
    if !insert_seal(
        &tx,
        &NewSeal {
            ballot_box,
            outcome: &outcome,
            signed_message: &signed_message,
            record,
        },
    )
    .await?
    {
        return Err(anyhow!(
            "Ballot box of election {} area {} got a seal while it was locked",
            ids.election_id,
            ids.area_id
        ));
    }
    let log_record = SealLogRecord::from(record);
    tx.commit().await.context(SealErrorCategory::Board)?;
    error!(seal_id = %seal.id, %reason, "The ballot box could not be sealed: it stays locked");
    if let Some(transaction) = pending_row(client, &seal.id).await? {
        mark_failed_with_names(&transaction, &seal.id, &reason, Some(&log_record), names).await?;
        transaction.commit().await?;
    }
    Ok(SealOutcome::Failed(reason))
}

/// The first channel, in the order the ballots come, that has a ballot in
/// the box and isn't CLOSED in the election's `status`; `None` when every
/// such channel is closed. A ballot whose channel is none of the voting
/// channels is an incident.
pub fn open_ballot_channel(
    ballots: &[BoxBallot],
    status: &ElectionStatus,
) -> std::result::Result<Option<VotingStatusChannel>, EntriesError> {
    let mut open = None;
    for ballot in ballots {
        let channel = VotingStatusChannel::from_str(&ballot.voting_channel).map_err(|_| {
            EntriesError::Incident(format!(
                "a ballot has the unknown voting channel {:?}",
                ballot.voting_channel
            ))
        })?;
        if open.is_none() && status.status_by_channel(channel) != VotingStatus::CLOSED {
            open = Some(channel);
        }
    }
    Ok(open)
}

/// How each ballot of the box counts, as the tally decides: each voter's
/// latest valid ballot (by acceptance order) is counted, with the voter's
/// census weight, if the voter is in the census, and not eligible
/// otherwise; an older valid ballot is replaced; a rejected one is
/// discarded. A ballot without content or Ballot ID, or whose Ballot ID
/// doesn't match its content, is an incident; one that can't be read, or
/// whose outcome is still pending, is retried.
pub fn entries(
    ballots: &[BoxBallot],
    census: &Census,
) -> std::result::Result<Vec<SealEntry>, EntriesError> {
    // Each voter's latest valid ballot.
    let mut latest: HashMap<&str, i64> = HashMap::new();
    for ballot in ballots
        .iter()
        .filter(|ballot| ballot.status == BallotStatus::Valid)
    {
        latest
            .entry(ballot.voter_id.as_str())
            .and_modify(|seq| *seq = (*seq).max(ballot.seq))
            .or_insert(ballot.seq);
    }
    let candidates: HashSet<i64> = latest.values().copied().collect();

    let mut entries = Vec::with_capacity(ballots.len());
    for ballot in ballots {
        if ballot.content.is_empty() || ballot.ballot_id.is_empty() {
            return Err(EntriesError::Incident(format!(
                "the ballot {} has no content or no Ballot ID",
                ballot.seq
            )));
        }
        let computed = computed_ballot_id(&ballot.content, &ballot.format).map_err(|error| {
            EntriesError::Retry(format!("the ballot {} can't be read: {error}", ballot.seq))
        })?;
        if computed != ballot.ballot_id {
            return Err(EntriesError::Incident(format!(
                "a ballot does not match its Ballot ID (stored {}, content hashes to {computed})",
                ballot.ballot_id
            )));
        }
        let (disposition, weight) = match ballot.status {
            BallotStatus::Rejected => (SealDisposition::Discarded, 0),
            BallotStatus::Pending => {
                return Err(EntriesError::Retry(format!(
                    "the ballot {} is still in progress",
                    ballot.seq
                )))
            }
            BallotStatus::Valid if !candidates.contains(&ballot.seq) => {
                (SealDisposition::Replaced, 0)
            }
            BallotStatus::Valid => match census.get(&ballot.voter_id) {
                Some(weight) => (SealDisposition::Counted, *weight),
                None => (SealDisposition::NotEligible, 0),
            },
        };
        entries.push(SealEntry {
            ballot_hash: ballot_hash(&ballot.content)
                .map_err(|error| EntriesError::Retry(error.to_string()))?,
            ballot_id: ballot.ballot_id.clone(),
            disposition,
            weight,
            channel: ballot.voting_channel.clone(),
        });
    }
    Ok(entries)
}

/// The Ballot ID of a stored ballot's content in `format`, as the cast
/// computes it.
pub fn computed_ballot_id(content: &str, format: &str) -> Result<String> {
    let format =
        BallotFormat::from_str(format).map_err(|_| anyhow!("unknown ballot format {format:?}"))?;
    Ok(match format {
        BallotFormat::HashableMultiBallot => {
            let signed: SignedHashableMultiBallot =
                deserialize_str(content).map_err(|error| anyhow!("{error}"))?;
            let hashable: HashableMultiBallot = (&signed).try_into()?;
            hash_multi_ballot(&hashable)?
        }
        BallotFormat::HashableBallot => {
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
    use electoral_log::seal::DEFAULT_SEAL_CHANNEL;
    use sequent_core::ballot::TYPES_VERSION;
    use std::io::Write;

    const SINGLE: &str = "hashable-ballot";

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
        let ballot_id = computed_ballot_id(&content, SINGLE).unwrap();
        (content, ballot_id)
    }

    #[test]
    fn an_election_status_that_does_not_parse_is_an_error_not_a_guess() {
        assert_eq!(election_status(None).unwrap(), ElectionStatus::default());
        let mut closed = ElectionStatus::default();
        closed.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::CLOSED);
        assert_eq!(
            election_status(Some(serde_json::to_value(&closed).unwrap())).unwrap(),
            closed
        );
        assert!(election_status(Some(serde_json::json!({"voting_status": "SHUT"}))).is_err());
        assert!(election_status(Some(serde_json::json!("closed"))).is_err());
    }

    fn row(voter: &str, status: BallotStatus, seed: &str, seq: i64) -> BoxBallot {
        let (content, ballot_id) = ballot(seed);
        BoxBallot {
            seq,
            voter_id: voter.to_string(),
            status,
            format: SINGLE.to_string(),
            content,
            ballot_id,
            voting_channel: DEFAULT_SEAL_CHANNEL.to_string(),
        }
    }

    fn disposition_of(entries: &[SealEntry], ballot: &BoxBallot) -> (SealDisposition, u64) {
        let entry = entries
            .iter()
            .find(|entry| entry.ballot_id == ballot.ballot_id)
            .unwrap();
        (entry.disposition, entry.weight)
    }

    #[test]
    fn the_delivery_ids_name_the_box() {
        let ballot_box = BallotBoxId {
            election_event_id: "e",
            election_id: "l",
            area_id: "a",
        };
        assert_eq!(seal_delivery_id(ballot_box), "ballot-box-seal:e:l:a");
        assert_eq!(
            failure_delivery_id(ballot_box),
            "ballot-box-seal-failed:e:l:a"
        );
    }

    #[test]
    fn a_ballot_of_a_channel_that_is_not_closed_is_named() {
        let mut status = ElectionStatus::default();
        status.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::CLOSED);
        let online = row("alice", BallotStatus::Valid, "a", 1);
        let mut kiosk = row("bob", BallotStatus::Rejected, "b", 2);
        kiosk.voting_channel = "KIOSK".to_string();
        assert_eq!(open_ballot_channel(&[online.clone()], &status), Ok(None));
        assert_eq!(
            open_ballot_channel(&[online.clone(), kiosk.clone()], &status),
            Ok(Some(VotingStatusChannel::KIOSK))
        );
        status.set_status_by_channel(VotingStatusChannel::KIOSK, VotingStatus::CLOSED);
        assert_eq!(open_ballot_channel(&[online, kiosk], &status), Ok(None));
        assert_eq!(open_ballot_channel(&[], &status), Ok(None));
    }

    #[test]
    fn a_ballot_of_an_unknown_channel_is_an_incident() {
        let status = ElectionStatus::default();
        let mut paper = row("alice", BallotStatus::Valid, "a", 1);
        paper.voting_channel = "PAPER".to_string();
        let open = row("bob", BallotStatus::Valid, "b", 2);
        // Even after a ballot of a channel that isn't closed.
        assert_eq!(
            open_ballot_channel(&[open, paper], &status),
            Err(EntriesError::Incident(
                "a ballot has the unknown voting channel \"PAPER\"".to_string()
            ))
        );
    }

    #[test]
    fn each_ballot_gets_its_disposition() {
        let census = Census::from([("alice".to_string(), 1), ("bob".to_string(), 3)]);
        let alice_old = row("alice", BallotStatus::Valid, "a1", 1);
        let alice_new = row("alice", BallotStatus::Valid, "a2", 4);
        let bob = row("bob", BallotStatus::Valid, "b1", 2);
        let carol = row("carol", BallotStatus::Valid, "c1", 3);
        let dave = row("dave", BallotStatus::Rejected, "d1", 5);
        let ballots = vec![
            alice_old.clone(),
            bob.clone(),
            carol.clone(),
            alice_new.clone(),
            dave.clone(),
        ];
        let entries = entries(&ballots, &census).unwrap();
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
            .find(|entry| entry.ballot_id == bob.ballot_id)
            .unwrap();
        assert_eq!(entry.ballot_hash, ballot_hash(&bob.content).unwrap());
    }

    #[test]
    fn the_latest_ballot_is_the_last_accepted_valid_one() {
        let census = Census::from([("alice".to_string(), 1)]);
        let first = row("alice", BallotStatus::Valid, "x", 1);
        let second = row("alice", BallotStatus::Valid, "y", 2);
        let rejected = row("alice", BallotStatus::Rejected, "z", 3);
        let ballots = vec![second.clone(), rejected.clone(), first.clone()];
        let entries = entries(&ballots, &census).unwrap();
        assert_eq!(
            disposition_of(&entries, &second).0,
            SealDisposition::Counted
        );
        assert_eq!(
            disposition_of(&entries, &first).0,
            SealDisposition::Replaced
        );
        assert_eq!(
            disposition_of(&entries, &rejected).0,
            SealDisposition::Discarded
        );
    }

    #[test]
    fn a_ballot_still_pending_is_retried() {
        let pending = row("alice", BallotStatus::Pending, "a1", 7);
        assert_eq!(
            entries(&[pending], &Census::new()).unwrap_err(),
            EntriesError::Retry("the ballot 7 is still in progress".to_string())
        );
    }

    #[test]
    fn a_ballot_id_mismatch_fails_the_seal() {
        let mut tampered = row("alice", BallotStatus::Valid, "a1", 1);
        tampered.ballot_id = ballot("other").1;
        let reason = entries(&[tampered], &Census::new()).unwrap_err();
        assert!(
            matches!(&reason, EntriesError::Incident(text) if text.starts_with("a ballot does not match its Ballot ID")),
            "{reason:?}"
        );
    }

    #[test]
    fn a_ballot_that_cant_be_read_is_retried_not_an_incident() {
        let mut unreadable = row("alice", BallotStatus::Valid, "a1", 1);
        // Another types version: the content doesn't convert.
        unreadable.content = unreadable
            .content
            .replace(&format!("\"version\":{TYPES_VERSION}"), "\"version\":999");
        let reason = entries(&[unreadable.clone()], &Census::new()).unwrap_err();
        assert!(
            matches!(&reason, EntriesError::Retry(text) if text.contains("can't be read")),
            "{reason:?}"
        );
        // So is one of a format the seal doesn't know.
        let mut unknown = row("alice", BallotStatus::Valid, "a1", 1);
        unknown.format = "paper".to_string();
        let reason = entries(&[unknown], &Census::new()).unwrap_err();
        assert!(
            matches!(&reason, EntriesError::Retry(text) if text.contains("can't be read")),
            "{reason:?}"
        );
    }

    #[test]
    fn the_format_decides_how_the_content_is_read() {
        let (content, ballot_id) = ballot("a1");
        assert_eq!(computed_ballot_id(&content, SINGLE).unwrap(), ballot_id);
        assert!(computed_ballot_id(&content, "hashable-multi-ballot").is_err());
    }

    #[test]
    fn a_ballot_without_content_or_id_fails_the_seal() {
        for missing_content in [true, false] {
            let mut ballot = row("alice", BallotStatus::Valid, "a1", 9);
            if missing_content {
                ballot.content = String::new();
            } else {
                ballot.ballot_id = String::new();
            }
            let reason = entries(&[ballot.clone()], &Census::new()).unwrap_err();
            assert_eq!(
                reason,
                EntriesError::Incident("the ballot 9 has no content or no Ballot ID".to_string())
            );
        }
    }

    #[test]
    fn an_empty_box_has_no_entries() {
        assert!(entries(&[], &Census::new()).unwrap().is_empty());
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
            let alice = row("alice", BallotStatus::Valid, "a", 1);
            let bob = row("bob", BallotStatus::Valid, "b", 2);
            let dave = row("dave", BallotStatus::Valid, "d", 3);
            let ballots = [alice.clone(), bob.clone(), dave.clone()];
            let sealed = entries(&ballots, &census).unwrap();

            // The tally's join over the same voters and the voters' latest
            // valid ballots, both sorted by voter id.
            let ballots_csv: String = [&alice, &bob, &dave]
                .iter()
                .map(|ballot| {
                    format!(
                        "{},{},{}\n",
                        ballot.voter_id, ballot.ballot_id, ballot.voting_channel
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
                assert_eq!(Some(&weight), tally.get(&ballot.ballot_id), "{column:?}");
            }
            assert_eq!(
                disposition_of(&sealed, &dave).0,
                SealDisposition::NotEligible
            );
            assert_eq!(joined.ballots_without_voter, 1, "{column:?}");
        }
    }

    #[test]
    fn privacy_seal_and_tally_preserve_weights_revotes_and_eligibility() {
        let census_csv = "AAAA-AAAA,7\nZZZZ-ZZZZ,2\n";
        let census =
            census_from_csv(census_csv.as_bytes(), VoterMultiplicityColumn::VoteWeight).unwrap();
        let superseded = row("AAAA-AAAA", BallotStatus::Valid, "old", 1);
        let current = row("AAAA-AAAA", BallotStatus::Valid, "new", 2);
        let other = row("ZZZZ-ZZZZ", BallotStatus::Valid, "other", 3);
        let disabled = row("MMMM-MMMM", BallotStatus::Valid, "disabled", 4);
        let sealed = entries(
            &[
                superseded.clone(),
                current.clone(),
                other.clone(),
                disabled.clone(),
            ],
            &census,
        )
        .unwrap();
        assert_eq!(
            disposition_of(&sealed, &superseded),
            (SealDisposition::Replaced, 0)
        );
        assert_eq!(
            disposition_of(&sealed, &disabled),
            (SealDisposition::NotEligible, 0)
        );
        let ballot_csv = [&current, &disabled, &other]
            .iter()
            .map(|ballot| format!("{},{}\n", ballot.voter_id, ballot.ballot_id))
            .collect::<String>();
        let joined = merge_join_csv(
            &file_with(&ballot_csv),
            &file_with(census_csv),
            0,
            0,
            1,
            None,
            Some(MultiplicitySource::VoteWeight(1)),
        )
        .unwrap();
        let weights: HashMap<String, u64> = joined.ballot_contents.into_iter().collect();
        assert_eq!(weights.get(&current.ballot_id), Some(&7));
        assert_eq!(weights.get(&other.ballot_id), Some(&2));
        assert_eq!(
            disposition_of(&sealed, &current),
            (SealDisposition::Counted, 7)
        );
        assert_eq!(
            disposition_of(&sealed, &other),
            (SealDisposition::Counted, 2)
        );
        assert_eq!(joined.ballots_without_voter, 1);
    }

    #[test]
    fn a_vote_weight_out_of_range_is_refused_like_the_tally() {
        assert!(
            census_from_csv("alice,0\n".as_bytes(), VoterMultiplicityColumn::VoteWeight).is_err()
        );
    }

    #[test]
    fn an_unrecorded_ballot_is_named_in_the_reason() {
        assert_eq!(
            unrecorded_reason(2),
            "2 ballot(s) of the ballot box have no cast-vote record on the bulletin board"
        );
    }
}
