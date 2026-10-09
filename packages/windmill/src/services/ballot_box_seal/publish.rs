// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Publishing a sealed ballot box (VOTE-FREEZE): deliver the stored signed
//! `BallotBoxSealed` message to the event's electoral log (idempotently),
//! read its entry id back, upload the seal record and mark the seal
//! `published`. The event's Seal Record Publication policy decides where
//! the record goes: Restricted (the default) keeps it a private event
//! document, Public puts it in the public bucket. Every step can run again:
//! the delivery is keyed by the seal, the record's document id and path are
//! fixed, and the row update happens in the same transaction as the document
//! row.

use super::record::build_record;
use super::{strict_presentation, with_timeout, SealEnvironment};
use crate::postgres::ballot_box_seal::{
    mark_published, try_lock, BallotBoxSealStatus, PublishedFields, TryLocked,
};
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager::get_board_client;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use deadpool_postgres::Client as DbClient;
use electoral_log::client::board_client::{
    ElectoralLogMessage, ElectoralLogVarCharColumn, SqlCompOperators, WhereClauseBTreeMap,
};
use electoral_log::messages::message::Message;
use electoral_log::messages::statement::StatementType;
use immudb_rs::TxMode;
use sequent_core::ballot::BallotBoxSealRecordPolicy;
use sequent_core::services::s3::get_public_election_event_document_name_key;
use sequent_core::types::hasura::core::ElectionEvent;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::StrandSignaturePk;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// How many `BallotBoxSealed` entries of a box one board read returns; the
/// reads page through all of them.
const SEALED_ENTRIES_PAGE: i64 = 100;
/// The most entries a box's read accepts before it fails loudly: anyone who
/// can write the log could post junk entries, and a genuine one must not be
/// hidden behind them.
const SEALED_ENTRIES_MAX: usize = 10_000;

/// What [`publish_box`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    NotFound,
    /// The seal isn't sealed: still pending, failed, or already published.
    NotSealed(BallotBoxSealStatus),
    /// Another run holds the seal; the next tick retries.
    Busy,
    Published,
}

/// Lowercase hex SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The event's electoral log (board database).
pub fn event_board(election_event: &ElectionEvent) -> Result<String> {
    get_election_event_board(election_event.bulletin_board_reference.clone())
        .ok_or_else(|| anyhow!("Election event {} has no bulletin board", election_event.id))
}

/// The seal record's document name; for a public record also its path
/// under the event's public folder.
pub fn record_name(election_id: &Uuid, area_id: &Uuid) -> String {
    format!("ballot-box-seals/{election_id}/{area_id}.json")
}

/// The seal record's document id, public or private: fixed per seal, so a
/// retry can't add a second document. Version-4 shaped, as the document
/// table requires.
pub fn record_document_id(seal_id: &Uuid) -> Uuid {
    let digest = Sha256::digest(format!("ballot-box-seal-record:{seal_id}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    uuid::Builder::from_random_bytes(bytes).into_uuid()
}

/// Where a seal record named `name` is in the public bucket: only a Public
/// record has a public path.
pub fn record_public_path(
    policy: BallotBoxSealRecordPolicy,
    tenant_id: &str,
    election_event_id: &str,
    name: &str,
) -> Option<String> {
    match policy {
        BallotBoxSealRecordPolicy::PUBLIC => Some(get_public_election_event_document_name_key(
            tenant_id,
            election_event_id,
            name,
        )),
        BallotBoxSealRecordPolicy::RESTRICTED => None,
    }
}

/// Delivers `message` to `board` once: a repeated `delivery_id` with the
/// same `payload_hash` is a no-op. Returns whether it inserted.
pub async fn deliver(
    board: &str,
    delivery_id: &str,
    payload_hash: &str,
    message: &Message,
) -> Result<bool> {
    with_timeout(
        "Posting to the electoral log",
        deliver_now(board, delivery_id, payload_hash, message),
    )
    .await
}

async fn deliver_now(
    board: &str,
    delivery_id: &str,
    payload_hash: &str,
    message: &Message,
) -> Result<bool> {
    let row = ElectoralLogMessage::try_from(message)?;
    let mut client = get_board_client().await?;
    client.open_session(board).await?;
    let result = async {
        client.ensure_electoral_log_delivery_receipts().await?;
        let immudb_tx = client.new_tx(TxMode::ReadWrite).await?;
        let inserted = client
            .insert_electoral_log_delivery(&immudb_tx, delivery_id, payload_hash, &[row])
            .await?;
        client.commit(&immudb_tx).await?;
        Ok(inserted)
    }
    .await;
    if let Err(error) = client.close_session().await {
        warn!(%board, ?error, "Error closing the electoral log session");
    }
    result
}

/// Every `BallotBoxSealed` entry of a ballot box in the event's log, in id
/// order, whoever signed it (see [`genuine_entries`]).
pub async fn sealed_entries(
    board: &str,
    election_id: &str,
    area_id: &str,
) -> Result<Vec<ElectoralLogMessage>> {
    let filter: WhereClauseBTreeMap = BTreeMap::from([
        (
            ElectoralLogVarCharColumn::StatementKind,
            (
                SqlCompOperators::Equal,
                StatementType::BallotBoxSealed.to_string(),
            ),
        ),
        (
            ElectoralLogVarCharColumn::ElectionId,
            (SqlCompOperators::Equal, election_id.to_string()),
        ),
        (
            ElectoralLogVarCharColumn::AreaId,
            (SqlCompOperators::Equal, area_id.to_string()),
        ),
    ]);
    with_timeout("Reading the electoral log", async {
        let mut client = get_board_client().await?;
        let mut entries = vec![];
        loop {
            let page = client
                .get_electoral_log_messages_filtered::<&str, &str>(
                    board,
                    Some(filter.clone()),
                    None,
                    None,
                    Some(SEALED_ENTRIES_PAGE),
                    Some(entries.len() as i64),
                    Some(std::collections::HashMap::from([("id", "asc")])),
                )
                .await
                .context("Error reading the ballot box's seal entries")?;
            let last_page = (page.len() as i64) < SEALED_ENTRIES_PAGE;
            entries.extend(page);
            if entries.len() > SEALED_ENTRIES_MAX {
                return Err(anyhow!(
                    "Ballot box of election {election_id} area {area_id} has more than \
                     {SEALED_ENTRIES_MAX} BallotBoxSealed entries on the bulletin board"
                ));
            }
            if last_page {
                return Ok(entries);
            }
        }
    })
    .await
}

/// The entries signed by `key` as sender and system: the event key's own
/// seals, not entries someone else posted.
pub fn genuine_entries(
    entries: Vec<ElectoralLogMessage>,
    key: &StrandSignaturePk,
) -> Vec<ElectoralLogMessage> {
    let Ok(key_der) = key.to_der() else {
        return vec![];
    };
    entries
        .into_iter()
        .filter(|entry| {
            Message::strand_deserialize(&entry.message)
                .ok()
                .and_then(|message| {
                    let signer = signing_key_of(&message).ok()?;
                    Some(signer.to_der().ok()? == key_der)
                })
                .unwrap_or(false)
        })
        .collect()
}

/// The key that signed a seal message as system: its sender's, checked
/// against the system signature (the event key signs as both).
pub fn signing_key_of(message: &Message) -> Result<StrandSignaturePk> {
    let statement = message.statement.strand_serialize()?;
    message
        .sender
        .pk
        .verify(&message.system_signature, &statement)
        .map_err(|error| {
            anyhow!("The seal message isn't signed by its sender as system: {error}")
        })?;
    Ok(message.sender.pk.clone())
}

/// Publishes the seal `seal_id` if it is sealed.
#[instrument(skip(client, environment), err)]
pub async fn publish_box(
    client: &mut DbClient,
    environment: &dyn SealEnvironment,
    seal_id: &Uuid,
) -> Result<PublishOutcome> {
    let transaction = client.transaction().await?;
    let seal = match try_lock(&transaction, seal_id).await? {
        TryLocked::Locked(seal) => seal,
        TryLocked::Busy => return Ok(PublishOutcome::Busy),
        TryLocked::NotFound => return Ok(PublishOutcome::NotFound),
    };
    if seal.status != BallotBoxSealStatus::Sealed {
        transaction.commit().await?;
        return Ok(PublishOutcome::NotSealed(seal.status));
    }
    let signed_message = seal
        .signed_message
        .clone()
        .ok_or_else(|| anyhow!("Sealed ballot box seal {seal_id} has no signed message"))?;
    let message = Message::strand_deserialize(&signed_message).map_err(|error| {
        anyhow!("The signed message of seal {seal_id} does not decode: {error}")
    })?;
    let tenant_id = seal.tenant_id.to_string();
    let election_event_id = seal.election_event_id.to_string();
    let election_event =
        get_election_event_by_id(&transaction, &tenant_id, &election_event_id).await?;
    let board = event_board(&election_event)?;

    // 1. The log entry, once per seal.
    deliver(
        &board,
        &sha256_hex(format!("ballot-box-seal:{}", seal.id).as_bytes()),
        &sha256_hex(&signed_message),
        &message,
    )
    .await?;

    // 2. Its entry id.
    let log_entry_id = sealed_entries(
        &board,
        &seal.election_id.to_string(),
        &seal.area_id.to_string(),
    )
    .await?
    .into_iter()
    .find(|entry| entry.message == signed_message)
    .map(|entry| entry.id)
    .ok_or_else(|| anyhow!("The seal entry of {seal_id} is not on the bulletin board yet"))?;

    // 3. The seal record, with its fixed document id (and path, when
    // public). Its key is the one that signed the message (sender and
    // system), not the key of today. The record policy is locked once voting
    // opened, so every box of the event is published the same way.
    let record_policy = strict_presentation(&election_event)?.ballot_box_seal_record_policy();
    let system_pk = signing_key_of(&message)?;
    let record = build_record(
        &transaction,
        &seal,
        &election_event,
        &system_pk,
        log_entry_id,
    )
    .await?;
    let json = serde_json::to_vec_pretty(&record)?;
    let name = record_name(&seal.election_id, &seal.area_id);
    let public_document_id = environment
        .upload_record(
            &transaction,
            &tenant_id,
            &election_event_id,
            &name,
            &json,
            record_document_id(&seal.id),
            record_policy,
        )
        .await?;

    // 4. Published.
    let fields = PublishedFields {
        log_entry_id,
        public_document_id,
        public_path: record_public_path(record_policy, &tenant_id, &election_event_id, &name),
        published_at: Utc::now(),
    };
    mark_published(&transaction, &seal.id, &fields).await?;
    transaction.commit().await?;
    info!(
        %seal_id,
        log_entry_id,
        %record_policy,
        %public_document_id,
        public_path = ?fields.public_path,
        "Published the ballot box seal"
    );
    Ok(PublishOutcome::Published)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_public_record_has_a_public_path() {
        let name = "ballot-box-seals/e/a.json";
        assert_eq!(
            record_public_path(BallotBoxSealRecordPolicy::PUBLIC, "t", "ev", name).as_deref(),
            Some("tenant-t/event-ev/ballot-box-seals/e/a.json")
        );
        assert_eq!(
            record_public_path(BallotBoxSealRecordPolicy::RESTRICTED, "t", "ev", name),
            None
        );
    }

    #[test]
    fn the_record_document_id_is_fixed_per_seal() {
        let seal = Uuid::from_u128(7);
        assert_eq!(record_document_id(&seal), record_document_id(&seal));
        assert_ne!(
            record_document_id(&seal),
            record_document_id(&Uuid::from_u128(8))
        );
        assert_eq!(record_document_id(&seal).get_version_num(), 4);
    }
}
