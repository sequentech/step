// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HSM outer encryption of stored cast votes.
//!
//! An election event that requires sealing (`cast_vote_sealing_required`)
//! stores every cast vote sealed to its HSM key in `hsm-seal-v1`, in both
//! `ballot_box_received` and `ballot_box_ballot`. Sealing needs only the
//! key's public point, so the cast seals in software and the HSM stays off
//! every voting path. Opening needs the HSM: an installation that has one
//! installs a [`CastVoteOpener`], which the tally's dump and the seal of a
//! ballot box (VOTE-FREEZE) open through. An event without the flag stores
//! and reads plaintext ballots exactly as before.
//!
//! - The key is published once per event, insert-only, and its fingerprint
//!   is pinned in the event's electoral log by a signed
//!   `CastVoteSealKeyPublished` statement ([`publish_key`]). A cast refuses
//!   a key that is not the pinned one.
//! - With the flag set, a missing key, a key that doesn't match its pin, a
//!   missing opener or a plaintext row is refused, never read as legacy.
//! - Every opened ballot must hash to the Ballot ID its row and envelope
//!   carry; the associated data binds each envelope to its row's tenant,
//!   event, election, area, Ballot ID and inner format.

pub mod format;

use crate::services::ballot_box_seal::seal::computed_ballot_id;
use crate::services::electoral_log_board::{get_board_identity, BoardIdentity};
use anyhow::{anyhow, ensure, Context, Result};
use dashmap::DashMap;
use deadpool_postgres::Transaction;
use electoral_log::cast_vote_seal::{
    associated_data, key_id as event_key_id, CastVoteSealKeyPin, SealScope, SealedBallot,
    ENVELOPE_VERSION, SEAL_FORMAT,
};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::statement::{StatementBody, StatementType};
use electoral_log::ports::ElectoralLogStore;
use electoral_log::{Filter, LogEntry, LogQuery, SqlCompOperators, TextColumn};
use once_cell::sync::Lazy;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sha2::{Digest, Sha256};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use voting_crypto::signing::VerificationKey;
use voting_crypto::RecordDeserialize;

/// Most ballots one call of the opener carries.
pub const MAX_OPEN_BATCH: usize = 64;
/// Most `CastVoteSealKeyPublished` entries a pin check reads.
const PIN_READ_LIMIT: i64 = 16;

/// One value to open: the `hsm-seal-v1` bytes and their associated data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenItem {
    pub sealed: Vec<u8>,
    pub aad: Vec<u8>,
}

/// The answer of [`CastVoteOpener::open_batch`]: one result per item, in
/// order.
pub type OpenResults = Vec<std::result::Result<Vec<u8>, String>>;
/// The future [`CastVoteOpener::open_batch`] returns.
pub type OpenFuture<'a> = Pin<Box<dyn Future<Output = Result<OpenResults>> + Send + 'a>>;

/// What an installation that holds the events' keys in an HSM provides to
/// open sealed cast votes.
pub trait CastVoteOpener: Send + Sync {
    /// Opens values sealed to `key_id` at `key_version`, at most
    /// [`MAX_OPEN_BATCH`] at a time: one result per item, in order. `Err` is
    /// an operational failure of the whole batch; an item's `Err` is that
    /// item not opening (wrong key, associated data or bytes).
    fn open_batch<'a>(
        &'a self,
        key_id: &'a str,
        key_version: u32,
        items: Vec<OpenItem>,
    ) -> OpenFuture<'a>;
}

static INSTALLED: RwLock<Option<Arc<dyn CastVoteOpener>>> = RwLock::new(None);

/// Installs the opener; done once, at start, by a build that provides one.
/// Without it, the ballots of a sealed event are not opened: its tally and
/// its ballot box seals are refused.
pub fn install(opener: Arc<dyn CastVoteOpener>) -> Result<()> {
    let mut installed = INSTALLED
        .write()
        .map_err(|_| anyhow!("the cast vote opener's lock is poisoned"))?;
    if installed.is_some() {
        return Err(anyhow!("a cast vote opener is installed already"));
    }
    *installed = Some(opener);
    Ok(())
}

pub fn installed() -> Result<Arc<dyn CastVoteOpener>> {
    INSTALLED
        .read()
        .map_err(|_| anyhow!("the cast vote opener's lock is poisoned"))?
        .clone()
        .ok_or_else(|| {
            anyhow!(
                "this election event's cast votes are sealed to an HSM key, and this installation has no opener: refused"
            )
        })
}

/// An election event's published key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealKey {
    pub key_id: String,
    pub key_version: u32,
    /// The uncompressed P-256 public point.
    pub public_key: Vec<u8>,
    /// Lowercase hex of SHA-256 of the public point.
    pub fingerprint: String,
}

impl SealKey {
    pub fn new(key_id: String, key_version: u32, public_key: Vec<u8>) -> Self {
        let fingerprint = hex::encode(Sha256::digest(&public_key));
        SealKey {
            key_id,
            key_version,
            public_key,
            fingerprint,
        }
    }

    /// The pin of this key for an event's electoral log.
    pub fn pin(&self, tenant_id: &str, election_event_id: &str) -> CastVoteSealKeyPin {
        CastVoteSealKeyPin {
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            key_id: self.key_id.clone(),
            key_version: self.key_version,
            format: SEAL_FORMAT.to_string(),
            public_key: hex::encode(&self.public_key),
            fingerprint: self.fingerprint.clone(),
        }
    }
}

/// Whether an election event's cast votes are sealed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sealing {
    /// The event does not require sealing: plaintext ballots, as always.
    Legacy,
    /// The event requires sealing; `None` until its key is published, and
    /// every cast is refused meanwhile.
    Required(Option<SealKey>),
}

impl Sealing {
    /// The key a sealed event's ballots are sealed to; `None` for a legacy
    /// event, and an error for a sealed one without a key.
    pub fn key(&self) -> Result<Option<&SealKey>> {
        match self {
            Sealing::Legacy => Ok(None),
            Sealing::Required(Some(key)) => Ok(Some(key)),
            Sealing::Required(None) => Err(anyhow!(
                "the election event requires sealed cast votes but its key is not published: refused"
            )),
        }
    }
}

/// The event's sealing state, as published.
pub async fn sealing(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Sealing> {
    let row = hasura_transaction
        .query_one(
            "SELECT required, key_id, key_version, public_key_sec1, format, fingerprint \
             FROM sequent_backend.cast_vote_seal_state($1, $2)",
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await
        .context("Error reading the event's cast vote sealing")?;
    let required: bool = row.try_get(0)?;
    if !required {
        return Ok(Sealing::Legacy);
    }
    let key_id: Option<String> = row.try_get(1)?;
    let Some(key_id) = key_id else {
        return Ok(Sealing::Required(None));
    };
    let key_version: i32 = row.try_get(2)?;
    let public_key: Vec<u8> = row.try_get(3)?;
    let format: String = row.try_get(4)?;
    let fingerprint: String = row.try_get(5)?;
    ensure!(
        format == SEAL_FORMAT,
        "The event's key has the unknown format {format}"
    );
    ensure!(
        key_id == event_key_id(tenant_id, election_event_id)?,
        "The event's key is another event's"
    );
    let key = SealKey::new(key_id, u32::try_from(key_version)?, public_key);
    ensure!(
        key.fingerprint == fingerprint,
        "The event's key does not match its fingerprint"
    );
    Ok(Sealing::Required(Some(key)))
}

/// What a sealed cast vote's row is bound to.
#[derive(Debug, Clone, Copy)]
pub struct RowScope<'a> {
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub election_id: &'a str,
    pub area_id: &'a str,
}

/// Seals a voter's ballot (`content`, of `inner_format`, with its Ballot ID
/// and cast-vote hash as the cast computed them) for its row.
pub fn seal_ballot(
    key: &SealKey,
    row: RowScope<'_>,
    inner_format: &str,
    ballot_id: &str,
    cast_vote_hash: &[u8],
    content: &str,
) -> Result<SealedBallot> {
    ensure!(
        key.key_id == event_key_id(row.tenant_id, row.election_event_id)?,
        "The key is not the election event's"
    );
    let aad = associated_data(&SealScope {
        tenant_id: row.tenant_id,
        election_event_id: row.election_event_id,
        election_id: row.election_id,
        area_id: row.area_id,
        ballot_id,
        inner_format,
    })?;
    let sealed = format::seal(
        &key.public_key,
        &key.key_id,
        key.key_version,
        content.as_bytes(),
        &aad,
    )?;
    let envelope = SealedBallot {
        v: ENVELOPE_VERSION,
        key_id: key.key_id.clone(),
        key_version: key.key_version,
        inner_format: inner_format.to_string(),
        ballot_id: ballot_id.to_string(),
        cast_vote_hash: hex::encode(cast_vote_hash),
        sealed: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, sealed),
    };
    envelope.check()?;
    Ok(envelope)
}

/// A stored sealed ballot to open, with what its row says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedRow {
    pub election_id: String,
    pub area_id: String,
    pub ballot_id: String,
    pub format: String,
    pub content: String,
}

/// Why sealed rows were not opened.
#[derive(Debug)]
pub enum OpenError {
    /// No opener is installed, or it failed for the whole batch: nothing is
    /// known about the ballots, and the same call may succeed later.
    Unavailable(anyhow::Error),
    /// A stored ballot doesn't open, isn't sealed as its row says, or doesn't
    /// hash to its Ballot ID: it was changed or moved.
    Ballot(String),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Unavailable(error) => write!(f, "{error:#}"),
            OpenError::Ballot(reason) => f.write_str(reason),
        }
    }
}

impl std::error::Error for OpenError {}

/// Opens sealed rows of an event through the installed opener, in batches,
/// and checks each opened ballot against its Ballot ID. Returns the voter
/// ballots in the rows' order.
pub async fn open_rows(
    tenant_id: &str,
    election_event_id: &str,
    key: &SealKey,
    rows: &[SealedRow],
) -> std::result::Result<Vec<String>, OpenError> {
    let opener = installed().map_err(OpenError::Unavailable)?;
    open_rows_with(opener.as_ref(), tenant_id, election_event_id, key, rows).await
}

/// The envelope of a sealed row and what opening it takes.
fn open_item(
    tenant_id: &str,
    election_event_id: &str,
    key: &SealKey,
    row: &SealedRow,
) -> Result<(SealedBallot, OpenItem)> {
    let envelope = SealedBallot::from_row(&row.content, &row.format, &row.ballot_id)?;
    ensure!(
        envelope.key_id == key.key_id && envelope.key_version == key.key_version,
        "The sealed ballot {} is sealed to another key",
        row.ballot_id
    );
    let aad = associated_data(&SealScope {
        tenant_id,
        election_event_id,
        election_id: &row.election_id,
        area_id: &row.area_id,
        ballot_id: &row.ballot_id,
        inner_format: &envelope.inner_format,
    })?;
    let sealed = envelope.sealed_bytes()?;
    Ok((envelope, OpenItem { sealed, aad }))
}

/// The voter ballot an opened row holds, once it hashes to its Ballot ID.
fn opened_ballot(
    row: &SealedRow,
    envelope: &SealedBallot,
    result: std::result::Result<Vec<u8>, String>,
) -> Result<String> {
    let bytes = result
        .map_err(|error| anyhow!("The sealed ballot {} does not open: {error}", row.ballot_id))?;
    let inner = String::from_utf8(bytes)
        .map_err(|_| anyhow!("The sealed ballot {} is not text", row.ballot_id))?;
    let computed = computed_ballot_id(&inner, &envelope.inner_format)
        .with_context(|| format!("The sealed ballot {} can't be read", row.ballot_id))?;
    ensure!(
        computed == row.ballot_id,
        "The sealed ballot {} does not match its Ballot ID (it hashes to {computed})",
        row.ballot_id
    );
    Ok(inner)
}

/// [`open_rows`] with a given opener.
pub async fn open_rows_with(
    opener: &dyn CastVoteOpener,
    tenant_id: &str,
    election_event_id: &str,
    key: &SealKey,
    rows: &[SealedRow],
) -> std::result::Result<Vec<String>, OpenError> {
    let ballot = |error: anyhow::Error| OpenError::Ballot(format!("{error:#}"));
    let mut opened = Vec::with_capacity(rows.len());
    for chunk in rows.chunks(MAX_OPEN_BATCH) {
        let (envelopes, items): (Vec<_>, Vec<_>) = chunk
            .iter()
            .map(|row| open_item(tenant_id, election_event_id, key, row))
            .collect::<Result<Vec<_>>>()
            .map_err(ballot)?
            .into_iter()
            .unzip();
        let results = opener
            .open_batch(&key.key_id, key.key_version, items)
            .await
            .context("The cast vote opener failed")
            .map_err(OpenError::Unavailable)?;
        if results.len() != chunk.len() {
            return Err(OpenError::Unavailable(anyhow!(
                "The cast vote opener returned {} results for {} ballots",
                results.len(),
                chunk.len()
            )));
        }
        for ((row, envelope), result) in chunk.iter().zip(&envelopes).zip(results) {
            opened.push(opened_ballot(row, envelope, result).map_err(ballot)?);
        }
    }
    Ok(opened)
}

/// Refuses rows whose format doesn't fit the event: a plaintext row in a
/// sealed event, or a sealed row in a legacy one.
pub fn check_row_format(sealing: &Sealing, format: &str, ballot_id: &str) -> Result<()> {
    let sealed = electoral_log::cast_vote_seal::is_sealed_format(format);
    match (sealing, sealed) {
        (Sealing::Legacy, false) | (Sealing::Required(_), true) => Ok(()),
        (Sealing::Legacy, true) => Err(anyhow!(
            "The ballot {ballot_id} is sealed, but its election event does not require sealing: refused"
        )),
        (Sealing::Required(_), false) => Err(anyhow!(
            "The ballot {ballot_id} is stored in plaintext in an election event that requires sealing: refused"
        )),
    }
}

/// The delivery ID of an event's `CastVoteSealKeyPublished` entry: one per
/// event on its board.
pub fn pin_delivery_id(election_event_id: &str) -> String {
    format!("cast-vote-seal-key:{election_event_id}")
}

/// The key pins on a board that its system key signed, distinct.
pub fn signed_pins(
    entries: &[electoral_log::ElectoralLogMessage],
    system_pk: &VerificationKey,
) -> Result<Vec<CastVoteSealKeyPin>> {
    let system_der = system_pk.to_der()?;
    let mut pins: Vec<CastVoteSealKeyPin> = Vec::new();
    for entry in entries {
        let Ok(message) = Message::record_deserialize(&entry.message) else {
            continue;
        };
        if message.verify(system_pk).is_err() || message.sender.pk.to_der()? != system_der {
            continue;
        }
        if let StatementBody::CastVoteSealKeyPublished(pin) = message.statement.body {
            if !pins.contains(&pin) {
                pins.push(pin);
            }
        }
    }
    Ok(pins)
}

/// The event's board pin, if any: an error when two different keys are
/// pinned.
pub async fn read_pin(
    store: &electoral_log::adapters::postgres::PostgresStore,
    board: &str,
    system_pk: &VerificationKey,
) -> Result<Option<CastVoteSealKeyPin>> {
    let query = LogQuery {
        filters: vec![Filter::Text(
            TextColumn::StatementKind,
            SqlCompOperators::Equal,
            StatementType::CastVoteSealKeyPublished.to_string(),
        )],
        limit: PIN_READ_LIMIT,
        ..LogQuery::default()
    };
    let entries = store
        .query(board, &query)
        .await
        .context("Error reading the cast vote key pin from the electoral log")?;
    let mut pins = signed_pins(&entries, system_pk)?;
    ensure!(
        pins.len() <= 1,
        "The electoral log pins {} different cast vote keys: refused",
        pins.len()
    );
    Ok(pins.pop())
}

/// Whether the published key is the pinned one.
pub fn check_pin(
    key: &SealKey,
    pin: Option<&CastVoteSealKeyPin>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let pin = pin.ok_or_else(|| {
        anyhow!("The election event's cast vote key is not pinned in its electoral log: refused")
    })?;
    ensure!(
        *pin == key.pin(tenant_id, election_event_id),
        "The election event's cast vote key is not the one its electoral log pins: refused"
    );
    Ok(())
}

/// The system key that signs an event's board.
pub async fn board_public_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board: &str,
) -> Result<VerificationKey> {
    match get_board_identity(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        board,
    )
    .await?
    {
        BoardIdentity::Software(key) => VerificationKey::from_sk(&key),
        BoardIdentity::External(key) => key.verification_key(),
    }
}

/// Pins checked once per process: a key and its pin never change.
static PINNED: Lazy<DashMap<(String, String), String>> = Lazy::new(DashMap::new);

/// The key a cast in the event seals to: `None` for a legacy event. A sealed
/// event whose key is missing or not the one its electoral log pins is an
/// error.
pub async fn cast_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event: &sequent_core::types::hasura::core::ElectionEvent,
) -> Result<Option<SealKey>> {
    let sealing = sealing(hasura_transaction, tenant_id, &election_event.id).await?;
    let Some(key) = sealing.key()?.cloned() else {
        return Ok(None);
    };
    let cache_key = (tenant_id.to_string(), election_event.id.clone());
    if PINNED.get(&cache_key).as_deref() == Some(&key.fingerprint) {
        return Ok(Some(key));
    }
    let board = crate::services::election_event_board::get_election_event_board(
        election_event.bulletin_board_reference.clone(),
    )
    .context("The election event has no electoral-log board")?;
    let system_pk =
        board_public_key(hasura_transaction, tenant_id, &election_event.id, &board).await?;
    let store = crate::services::electoral_log_board::get_event_store(&election_event.id).await?;
    let pin = read_pin(&store, &board, &system_pk).await?;
    check_pin(&key, pin.as_ref(), tenant_id, &election_event.id)?;
    PINNED.insert(cache_key, key.fingerprint.clone());
    Ok(Some(key))
}

/// Publishes an event's key and pins it in the event's electoral log. Run
/// by the installation's publisher, in a transaction of a login that holds
/// `sequent_os_cast_vote_key_publisher`, after it made the key in its HSM.
/// The database refuses it once the event opened voting without requiring
/// sealing first, and refuses another key for the event. Idempotent for the
/// same key. The caller commits.
pub async fn publish_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    key: &SealKey,
) -> Result<()> {
    ensure!(
        key.key_id == event_key_id(tenant_id, election_event_id)?,
        "The key is not the election event's"
    );
    let version = i32::try_from(key.key_version)?;
    let published: bool = hasura_transaction
        .query_one(
            "SELECT sequent_backend.cast_vote_seal_publish($1, $2, $3, $4, $5)",
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &key.key_id,
                &version,
                &key.public_key,
            ],
        )
        .await
        .context("Error publishing the cast vote key")?
        .try_get(0)?;
    ensure!(
        published,
        "The cast vote key was refused: the event has another key, or opened voting without requiring sealing"
    );
    let event = crate::postgres::election_event::get_election_event_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await?;
    let board = crate::services::election_event_board::get_election_event_board(
        event.bulletin_board_reference,
    )
    .context("The election event has no electoral-log board")?;
    let signer = crate::services::electoral_log_board::get_board_signer(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &board,
    )
    .await?;
    let system_pk = signer.public_key()?;
    let store = crate::services::electoral_log_board::get_event_store(election_event_id).await?;
    if let Some(pin) = read_pin(&store, &board, &system_pk).await? {
        return check_pin(key, Some(&pin), tenant_id, election_event_id);
    }
    let scope = crate::services::electoral_log_deferral::scope_key(tenant_id, election_event_id);
    let admission = crate::services::electoral_log::ScopeAdmission::admit(&scope).await?;
    let message = Message::cast_vote_seal_key_published_message(
        key.pin(tenant_id, election_event_id),
        &SigningData::with_signers(signer.clone(), "", signer),
    )?;
    let entry = LogEntry {
        delivery_id: pin_delivery_id(election_event_id),
        message: (&message).try_into()?,
    };
    store
        .append(&board, &mut std::iter::once(Ok(entry)))
        .await
        .context("Error pinning the cast vote key in the electoral log")?;
    admission.end().await;
    let pin = read_pin(&store, &board, &system_pk).await?;
    check_pin(key, pin.as_ref(), tenant_id, election_event_id)
}

#[cfg(test)]
#[path = "cast_vote_seal_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) mod testing;
