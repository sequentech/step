// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Publishing a sealed ballot box (VOTE-FREEZE): the seal and its signed
//! `BallotBoxSealed` record are already in the event's electoral-log
//! database; publishing checks that the record on the board is the row's,
//! uploads the seal record and marks the seal `published`. The event's Seal
//! Record Publication policy decides where the record goes: Restricted (the
//! default) keeps it a private event document, Public puts it in the public
//! bucket. Every step can run again: the record's document id and path are
//! fixed, and the row update happens in the same transaction as the document
//! row.

use super::record::build_record;
use super::{strict_presentation, with_timeout, SealEnvironment};
use crate::postgres::ballot_box_seal::{
    mark_published, try_lock, BallotBoxSealStatus, PublishedFields, TryLocked,
};
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::election_event_board::get_election_event_board;
use anyhow::{anyhow, Result};
use chrono::Utc;
use deadpool_postgres::Client as DbClient;
use electoral_log::messages::message::Message;
use electoral_log::seal::SealRecordLogEntry;
use sequent_core::ballot::BallotBoxSealRecordPolicy;
use sequent_core::services::s3::get_public_election_event_document_name_key;
use sequent_core::types::hasura::core::ElectionEvent;
use sha2::{Digest, Sha256};
use tracing::{info, instrument};
use uuid::Uuid;
use voting_crypto::signing::VerificationKey;
use voting_crypto::{RecordDeserialize, RecordSerialize};

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

/// The key that signed a seal message as system: its sender's, checked
/// against the system signature (the event key signs as both).
pub fn signing_key_of(message: &Message) -> Result<VerificationKey> {
    let statement = message.statement.record_serialize()?;
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
    let message = Message::record_deserialize(&signed_message).map_err(|error| {
        anyhow!("The signed message of seal {seal_id} does not decode: {error}")
    })?;
    let tenant_id = seal.tenant_id.to_string();
    let election_event_id = seal.election_event_id.to_string();
    let election_event =
        get_election_event_by_id(&transaction, &tenant_id, &election_event_id).await?;
    let board = event_board(&election_event)?;

    // 1. The seal's record on the board, which the seal stored with it.
    let log_record = seal
        .log_record
        .clone()
        .ok_or_else(|| anyhow!("Sealed ballot box seal {seal_id} has no log record"))?;
    let delivered = with_timeout("Reading the electoral log", async {
        environment
            .event_store(&election_event_id)
            .await?
            .delivered(&board, &log_record.delivery_id)
            .await
    })
    .await?
    .ok_or_else(|| {
        anyhow!(
            "The seal record {} of {seal_id} is not on the bulletin board",
            log_record.delivery_id
        )
    })?;
    if delivered.message.message != signed_message
        || delivered.record.log_uid != log_record.log_uid
        || delivered.record.leaf_index != log_record.leaf_index
    {
        return Err(anyhow!(
            "The seal record {} on the bulletin board is not the one of seal {seal_id}",
            log_record.delivery_id
        ));
    }
    let log_entry = SealRecordLogEntry::sealed(&delivered.record);

    // 2. The seal record, with its fixed document id (and path, when
    // public). Its key is the one that signed the message (sender and
    // system), not the key of today. The record policy is locked once voting
    // opened, so every box of the event is published the same way.
    let record_policy = strict_presentation(&election_event)?.ballot_box_seal_record_policy();
    let system_pk = signing_key_of(&message)?;
    let record = build_record(&transaction, &seal, &election_event, &system_pk, log_entry).await?;
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

    // 3. Published.
    let fields = PublishedFields {
        public_document_id,
        public_path: record_public_path(record_policy, &tenant_id, &election_event_id, &name),
        published_at: Utc::now(),
    };
    mark_published(&transaction, &seal.id, &fields).await?;
    transaction.commit().await?;
    info!(
        %seal_id,
        log_delivery_id = %log_record.delivery_id,
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
