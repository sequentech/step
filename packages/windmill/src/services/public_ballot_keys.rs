// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Public verification material outlives signing secrets. Reads never open the vault.
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::adapters::transfer::PublicBallotVerificationKey;
use electoral_log::proofs::{verify_checkpoint_signature, Checkpoint};
use sequent_core::ballot::BallotBoxKey;
use sequent_core::ballot_receipt::ballot_box_key;
use sequent_core::types::hasura::core::ElectionEvent;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
use voting_crypto::signing::VerificationKey;

pub fn validate(key: &PublicBallotVerificationKey) -> Result<()> {
    Uuid::parse_str(&key.election_event_id)?;
    VerificationKey::from_b64_string(&key.system_public_key)?;
    let receipt = ballot_box_key(&VerificationKey::from_b64_string(
        &key.ballot_box_public_key,
    )?)?;
    ensure!(
        receipt.key_id == key.ballot_box_key_id,
        "Receipt key fingerprint mismatch"
    );
    ensure!(
        electoral_log::adapters::events::event_of_board(&key.board_name)? == key.election_event_id,
        "Verification key board belongs to another event"
    );
    Ok(())
}

pub async fn retain(
    tx: &Transaction<'_>,
    tenant: &str,
    owner_event: &str,
    key: &PublicBallotVerificationKey,
) -> Result<()> {
    validate(key)?;
    let inserted = tx.execute(
        "INSERT INTO sequent_backend.ballot_verification_key(tenant_id,election_event_id,source_election_event_id,board_log_uid,board_name,ballot_box_key_id,system_public_key,ballot_box_public_key)
         VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING",
        &[&tenant,&owner_event,&key.election_event_id,&key.board_log_uid,&key.board_name,&key.ballot_box_key_id,&key.system_public_key,&key.ballot_box_public_key],
    ).await?;
    if inserted == 0 {
        let existing = list(tx, tenant, owner_event)
            .await?
            .into_iter()
            .find(|existing| {
                existing.board_log_uid == key.board_log_uid
                    && existing.ballot_box_key_id == key.ballot_box_key_id
            })
            .context("Retained verification key disappeared")?;
        ensure!(
            existing == *key,
            "A different public verification key is already retained"
        );
    }
    Ok(())
}

pub async fn list(
    tx: &Transaction<'_>,
    tenant: &str,
    owner_event: &str,
) -> Result<Vec<PublicBallotVerificationKey>> {
    let rows=tx.query("SELECT source_election_event_id::text,board_log_uid,board_name,ballot_box_key_id,system_public_key,ballot_box_public_key FROM sequent_backend.ballot_verification_key WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY board_log_uid,ballot_box_key_id", &[&tenant,&owner_event]).await?;
    rows.into_iter()
        .map(|row| {
            let key = PublicBallotVerificationKey {
                election_event_id: row.try_get(0)?,
                board_log_uid: row.try_get(1)?,
                board_name: row.try_get(2)?,
                ballot_box_key_id: row.try_get(3)?,
                system_public_key: row.try_get(4)?,
                ballot_box_public_key: row.try_get(5)?,
            };
            validate(&key)?;
            Ok(key)
        })
        .collect()
}

/// Called only while generating a publication, before its public files are exposed.
pub async fn for_publication(
    tx: &Transaction<'_>,
    event: &ElectionEvent,
    receipt: &BallotBoxKey,
) -> Result<()> {
    let board = super::election_event_board::get_election_event_board(
        event.bulletin_board_reference.clone(),
    )
    .context("Event board unavailable")?;
    let checkpoint = super::electoral_log_board::get_event_store(&event.id)
        .await?
        .journal()
        .checkpoint(&board)
        .await?;
    let signing = super::electoral_log_board::get_board_signing_key(
        tx,
        &event.tenant_id,
        Some(&event.id),
        &board,
    )
    .await?;
    retain(
        tx,
        &event.tenant_id,
        &event.id,
        &PublicBallotVerificationKey {
            election_event_id: event.id.clone(),
            board_log_uid: checkpoint.log_uid,
            board_name: board,
            ballot_box_key_id: receipt.key_id.clone(),
            ballot_box_public_key: receipt.public_key.clone(),
            system_public_key: VerificationKey::from_sk(&signing)?.to_b64_string()?,
        },
    )
    .await
}

/// Existing keys may be read once by an explicit migration, never by proof reads.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackfillSource {
    PublishedOnly,
    ExistingSigningKey,
}

/// One event per task, at most 1,000 distinct published key records. Historical
/// signed bytes are read unchanged; incomplete or contradictory anchors fail.
pub async fn backfill(
    tx: &Transaction<'_>,
    event: &ElectionEvent,
    source: BackfillSource,
) -> Result<usize> {
    let board = super::election_event_board::get_election_event_board(
        event.bulletin_board_reference.clone(),
    )
    .context("Event board unavailable")?;
    let store = super::electoral_log_board::get_event_store(&event.id).await?;
    let checkpoint = store.journal().checkpoint(&board).await?;
    let rows=tx.query("SELECT DISTINCT s.ballot_eml::jsonb->'ballot_box_key' FROM sequent_backend.ballot_style s JOIN sequent_backend.ballot_publication p ON p.id=s.ballot_publication_id AND p.tenant_id=s.tenant_id AND p.election_event_id=s.election_event_id WHERE s.tenant_id=$1::text::uuid AND s.election_event_id=$2::text::uuid AND p.published_at IS NOT NULL AND p.is_generated IS TRUE AND s.ballot_eml IS NOT NULL LIMIT 1001", &[&event.tenant_id,&event.id]).await?;
    ensure!(
        rows.len() <= 1000,
        "Too many publication keys for one backfill"
    );
    let mut receipts = BTreeMap::new();
    for row in rows {
        if let Some(value) = row.try_get::<_, Option<serde_json::Value>>(0)? {
            if value.is_null() {
                continue;
            }
            let receipt: BallotBoxKey = serde_json::from_value(value)?;
            ensure!(
                ballot_box_key(&VerificationKey::from_b64_string(&receipt.public_key)?)? == receipt,
                "Published receipt key fingerprint mismatch"
            );
            if let Some(existing) = receipts.insert(receipt.key_id.clone(), receipt.clone()) {
                ensure!(existing == receipt, "Conflicting published receipt keys");
            }
        }
    }
    if receipts.is_empty() && matches!(source, BackfillSource::ExistingSigningKey) {
        let signing = super::ballot_box_key::get_ballot_box_signing_key(tx, event)
            .await?
            .context("No retained publication or existing receipt key")?;
        let receipt = super::ballot_box_key::published_key(&signing)?;
        let different:bool=store.client().await?.query_one("SELECT EXISTS(SELECT 1 FROM ballot_box_received WHERE election_event_id=$1::text::uuid AND key_id<>$2)",&[&event.id,&receipt.key_id]).await?.get(0);
        ensure!(
            !different,
            "Existing receipt key does not cover historical receipts"
        );
        receipts.insert(receipt.key_id.clone(), receipt);
    }
    ensure!(
        !receipts.is_empty(),
        "No retained published receipt key; explicit recovery required"
    );
    let historical = store.client().await?.query(
        "SELECT DISTINCT key_id FROM ballot_box_received WHERE election_event_id=$1::text::uuid LIMIT 1001",
        &[&event.id],
    ).await?;
    ensure!(
        historical.len() <= 1000,
        "Too many historical receipt keys for one backfill"
    );
    for row in historical {
        let key_id: String = row.try_get(0)?;
        ensure!(
            receipts.contains_key(&key_id),
            "A historical receipt has no retained published key; explicit recovery required"
        );
    }
    let published = crate::postgres::electoral_log_checkpoint::get_electoral_log_checkpoints(
        tx,
        &event.tenant_id,
        &event.id,
    )
    .await?;
    let mut system = None;
    for anchor in published
        .into_iter()
        .filter(|anchor| anchor.log_uid == checkpoint.log_uid)
    {
        ensure!(
            anchor.board_name == board,
            "Checkpoint board identity mismatch"
        );
        let checkpoint = Checkpoint {
            log_name: anchor.board_name,
            log_uid: anchor.log_uid,
            tree_size: anchor.tree_size.try_into()?,
            root: hex::decode(anchor.root)?,
        };
        verify_checkpoint_signature(
            &checkpoint,
            anchor
                .reason
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid checkpoint reason"))?,
            &anchor.signer_pk,
            &anchor.signature,
        )?;
        if let Some(previous) = system.replace(anchor.signer_pk.clone()) {
            ensure!(
                previous == anchor.signer_pk,
                "Conflicting retained authority keys"
            );
        }
    }
    let system = match system {
        Some(key) => key,
        None if matches!(source, BackfillSource::ExistingSigningKey) => VerificationKey::from_sk(
            &super::electoral_log_board::get_board_signing_key(
                tx,
                &event.tenant_id,
                Some(&event.id),
                &board,
            )
            .await?,
        )?
        .to_b64_string()?,
        None => anyhow::bail!("No retained authority anchor; explicit recovery required"),
    };
    for receipt in receipts.values() {
        retain(
            tx,
            &event.tenant_id,
            &event.id,
            &PublicBallotVerificationKey {
                election_event_id: event.id.clone(),
                board_log_uid: checkpoint.log_uid,
                board_name: board.clone(),
                ballot_box_key_id: receipt.key_id.clone(),
                ballot_box_public_key: receipt.public_key.clone(),
                system_public_key: system.clone(),
            },
        )
        .await?;
    }
    Ok(receipts.len())
}
