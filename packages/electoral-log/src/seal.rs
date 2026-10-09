// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot box seal (VOTE-FREEZE): the manifest of a sealed ballot box, its
//! hash, the seal record (public or restricted) and the pure checks that
//! windmill's sealer, the tally and `step-cli verify-ballot-box-seal` share.
//!
//! A ballot box is the `cast_vote` rows of one (tenant, event, election,
//! area). The manifest lists every one of them, in any status, and its
//! SHA-512 (the seal hash) goes to the event's electoral log in a signed
//! `BallotBoxSealed` statement.

use std::collections::BTreeMap;
use std::fmt;

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use strand::hash::{hash_sha256, hash_to_array, Hash};
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::StrandSignaturePk;
use strum_macros::Display;

use crate::messages::message::Message;
use crate::messages::statement::{StatementBody, StatementType};

/// The manifest format this module writes and reads.
pub const SEAL_FORMAT_V1: &str = "step/ballot-box-seal/v1";

/// The channel recorded for a ballot whose annotations name none.
pub const DEFAULT_SEAL_CHANNEL: &str = "ONLINE";

/// Bytes of SHA-256(public key DER) a key fingerprint shows: 128 bits, so
/// grinding a key to match it is out of reach.
const FINGERPRINT_BYTES: usize = 16;
/// Hex digits per group in a key fingerprint (`AAA1 D55F 6B1D 555E`).
const FINGERPRINT_GROUP_DIGITS: usize = 4;
/// Ballot ids a diff description lists before it says "and N more".
const DIFF_IDS_SHOWN: usize = 5;

/// How a ballot of the box counts. The Borsh tag is signed: append only.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Display,
)]
#[serde(rename_all = "kebab-case")]
pub enum SealDisposition {
    /// The voter's latest valid ballot, and the voter is eligible.
    #[strum(serialize = "counted")]
    Counted,
    /// A valid ballot replaced by the same voter's later one.
    #[strum(serialize = "replaced")]
    Replaced,
    /// The voter's latest valid ballot, but the voter is not eligible.
    #[strum(serialize = "not-eligible")]
    NotEligible,
    /// A ballot with status `discarded`.
    #[strum(serialize = "discarded")]
    Discarded,
}

/// One ballot of the box. Entries are ordered by every field in turn, which
/// is (ballot_hash, ballot_id) first; duplicates are kept.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct SealEntry {
    /// SHA-512 of the stored `content` text's UTF-8 bytes, exactly as stored
    /// (see [`ballot_hash`]). Hex in JSON.
    #[serde(with = "hash_hex")]
    pub ballot_hash: Hash,
    /// The stored `cast_vote.ballot_id`, verbatim.
    pub ballot_id: String,
    pub disposition: SealDisposition,
    /// Multiplicity for a counted ballot (1, the delegate count or the vote
    /// weight); 0 for every other disposition.
    pub weight: u64,
    /// The ballot's voting channel (`annotations.voting_channel`), defaulting
    /// to [`DEFAULT_SEAL_CHANNEL`].
    pub channel: String,
}

/// The sealed contents of one ballot box. Its Borsh bytes are what is hashed
/// and stored; field order is part of the format.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BallotBoxSealManifest {
    /// [`SEAL_FORMAT_V1`].
    pub format: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub area_id: String,
    /// Unix seconds.
    pub closed_at: u64,
    pub grace_deadline: u64,
    pub sealed_at: u64,
    pub close_request_id: Option<String>,
    /// The census at the seal: enabled voters of the area authorized for the
    /// election.
    pub eligible_voters: u64,
    /// Sorted, see [`build`].
    pub entries: Vec<SealEntry>,
}

impl BallotBoxSealManifest {
    /// Every ballot of the box, in any status.
    pub fn ballots_in_box(&self) -> u64 {
        self.entries.len() as u64
    }

    /// The entries that count, without their weights: the statement's and
    /// the card's "ballots counted".
    pub fn ballots_counted(&self) -> u64 {
        self.count(SealDisposition::Counted)
    }

    pub fn count(&self, disposition: SealDisposition) -> u64 {
        self.entries
            .iter()
            .filter(|entry| entry.disposition == disposition)
            .count() as u64
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.strand_serialize()?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self::strand_deserialize(bytes)?)
    }
}

/// A manifest with its entries sorted, its exact bytes and its seal hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltSeal {
    pub manifest: BallotBoxSealManifest,
    pub bytes: Vec<u8>,
    pub hash: Hash,
}

/// SHA-512 of a stored ballot's `content`, as the manifest records it.
pub fn ballot_hash(content: &str) -> Result<Hash> {
    Ok(hash_to_array(content.as_bytes())?)
}

/// SHA-512 of the manifest bytes.
pub fn seal_hash(manifest_bytes: &[u8]) -> Result<Hash> {
    Ok(hash_to_array(manifest_bytes)?)
}

/// Sorts the entries, serializes the manifest and hashes it. The caller
/// sets every other field, the format included.
pub fn build(mut manifest: BallotBoxSealManifest) -> Result<BuiltSeal> {
    manifest.entries.sort();
    let bytes = manifest.to_bytes()?;
    let hash = seal_hash(&bytes)?;
    Ok(BuiltSeal {
        manifest,
        bytes,
        hash,
    })
}

/// The first 16 bytes of SHA-256 over the key's SPKI DER (the bytes whose
/// base64 the record carries), as 8 groups of 4 uppercase hex digits:
/// `AAA1 D55F 6B1D 555E 0C3A 9F21 7B44 E0D2`.
pub fn fingerprint(pk: &StrandSignaturePk) -> Result<String> {
    let digest = hash_sha256(&pk.to_der()?)?;
    let hex = hex::encode_upper(&digest[..FINGERPRINT_BYTES]);
    Ok(hex
        .as_bytes()
        .chunks(FINGERPRINT_GROUP_DIGITS)
        .map(|group| String::from_utf8_lossy(group).into_owned())
        .collect::<Vec<_>>()
        .join(" "))
}

/// What a stored ballot box has that its manifest doesn't, and the other
/// way round, compared as multisets of (ballot hash, ballot id).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SealDiff {
    /// In the manifest, not stored.
    pub missing: Vec<(Hash, String)>,
    /// Stored, not in the manifest.
    pub extra: Vec<(Hash, String)>,
}

impl SealDiff {
    pub fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.extra.is_empty()
    }

    /// A short English text for `TallyBallotBoxRejected`, such as
    /// "2 sealed ballots are missing (Ballot IDs a1…, b2…); 1 ballot is not in the seal (c3…)".
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        if !self.missing.is_empty() {
            parts.push(format!(
                "{} sealed {} missing ({})",
                self.missing.len(),
                if self.missing.len() == 1 {
                    "ballot is"
                } else {
                    "ballots are"
                },
                list_ids(&self.missing)
            ));
        }
        if !self.extra.is_empty() {
            parts.push(format!(
                "{} {} not in the seal ({})",
                self.extra.len(),
                if self.extra.len() == 1 {
                    "ballot is"
                } else {
                    "ballots are"
                },
                list_ids(&self.extra)
            ));
        }
        parts.join("; ")
    }
}

fn list_ids(ballots: &[(Hash, String)]) -> String {
    let shown: Vec<&str> = ballots
        .iter()
        .take(DIFF_IDS_SHOWN)
        .map(|(_, id)| id.as_str())
        .collect();
    let more = ballots.len().saturating_sub(DIFF_IDS_SHOWN);
    let mut text = format!("Ballot IDs {}", shown.join(", "));
    if more > 0 {
        text.push_str(&format!(" and {more} more"));
    }
    text
}

/// Compares the manifest's entries with the (ballot hash, ballot id) of
/// every stored row of the box. Empty when they are the same multiset.
pub fn compare_stored(manifest: &BallotBoxSealManifest, stored: &[(Hash, String)]) -> SealDiff {
    let mut balance: BTreeMap<(Hash, String), i64> = BTreeMap::new();
    for entry in &manifest.entries {
        *balance
            .entry((entry.ballot_hash, entry.ballot_id.clone()))
            .or_default() += 1;
    }
    for key in stored {
        *balance.entry(key.clone()).or_default() -= 1;
    }
    let mut diff = SealDiff::default();
    for (key, count) in balance {
        let target = if count > 0 {
            &mut diff.missing
        } else {
            &mut diff.extra
        };
        for _ in 0..count.unsigned_abs() {
            target.push(key.clone());
        }
    }
    diff
}

// ---------------------------------------------------------------------------
// Seal record (one JSON file per ballot box, public or restricted)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SealRecordNamed {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SealRecordBallots {
    pub in_the_box: u64,
    pub counted: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SealRecordSigner {
    pub name: String,
    /// The signer's certificate SHA-256, as the signing request stores it.
    pub certificate_sha256: String,
}

/// The Close voting request that closed the election, when signatures did.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SealRecordCloseRequest {
    pub id: String,
    pub signing_code: Option<String>,
    pub signers: Vec<SealRecordSigner>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SealRecordLogEntry {
    /// The electoral log entry id of the `BallotBoxSealed` message.
    pub id: i64,
    pub statement_kind: String,
}

/// The seal record of one ballot box: a public or a restricted (private)
/// document, by the event's Seal Record Publication policy. It holds no
/// voter id, no pseudonym, no time of an individual ballot and no network
/// data.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SealRecord {
    pub format: String,
    pub tenant_id: String,
    pub election_event: SealRecordNamed,
    pub election: SealRecordNamed,
    pub area: SealRecordNamed,
    /// RFC 3339 UTC to the second, e.g. `2028-05-08T11:01:12Z`.
    pub closed_at: String,
    pub grace_deadline: String,
    pub sealed_at: String,
    pub eligible_voters: u64,
    pub ballots: SealRecordBallots,
    pub close_request: Option<SealRecordCloseRequest>,
    /// Hex SHA-512 of the manifest.
    pub seal_hash: String,
    pub log_entry: SealRecordLogEntry,
    /// Base64 SPKI DER of the key that signs the event's electoral log.
    pub system_public_key: String,
    /// The signed message, readable. Must equal `message_b64` decoded.
    pub message: serde_json::Value,
    /// The signed message's Borsh bytes, base64, exactly as the log stores
    /// them (`ElectoralLogMessage.message`).
    pub message_b64: String,
    /// The manifest's entries, in manifest order.
    pub entries: Vec<SealEntry>,
    /// The manifest's Borsh bytes, base64.
    pub manifest: String,
}

/// The names a record shows next to the ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealRecordNames {
    pub election_event: String,
    pub election: String,
    pub area: String,
}

impl SealRecord {
    /// Builds the record from the seal row's stored bytes: the manifest and
    /// the signed message, both exactly as stored.
    pub fn from_parts(
        manifest_bytes: &[u8],
        signed_message: &[u8],
        system_pk: &StrandSignaturePk,
        names: SealRecordNames,
        close_request: Option<SealRecordCloseRequest>,
        log_entry_id: i64,
    ) -> Result<SealRecord> {
        let manifest = BallotBoxSealManifest::from_bytes(manifest_bytes)
            .context("the stored manifest does not decode")?;
        let message = Message::strand_deserialize(signed_message)
            .map_err(|error| anyhow!("the stored message does not decode: {error}"))?;
        Ok(SealRecord {
            format: manifest.format.clone(),
            tenant_id: manifest.tenant_id.clone(),
            election_event: SealRecordNamed {
                id: manifest.election_event_id.clone(),
                name: names.election_event,
            },
            election: SealRecordNamed {
                id: manifest.election_id.clone(),
                name: names.election,
            },
            area: SealRecordNamed {
                id: manifest.area_id.clone(),
                name: names.area,
            },
            closed_at: format_time(manifest.closed_at)?,
            grace_deadline: format_time(manifest.grace_deadline)?,
            sealed_at: format_time(manifest.sealed_at)?,
            eligible_voters: manifest.eligible_voters,
            ballots: SealRecordBallots {
                in_the_box: manifest.ballots_in_box(),
                counted: manifest.ballots_counted(),
            },
            close_request,
            seal_hash: hex::encode(seal_hash(manifest_bytes)?),
            log_entry: SealRecordLogEntry {
                id: log_entry_id,
                statement_kind: StatementType::BallotBoxSealed.to_string(),
            },
            system_public_key: system_pk.to_der_b64_string()?,
            message: serde_json::to_value(&message)?,
            message_b64: general_purpose::STANDARD.encode(signed_message),
            entries: manifest.entries.clone(),
            manifest: general_purpose::STANDARD.encode(manifest_bytes),
        })
    }
}

/// Unix seconds as the record writes them: `2028-05-08T11:01:12Z`.
pub fn format_time(unix_seconds: u64) -> Result<String> {
    let seconds = i64::try_from(unix_seconds)?;
    let time = DateTime::<Utc>::from_timestamp(seconds, 0)
        .ok_or_else(|| anyhow!("{unix_seconds} is not a representable time"))?;
    Ok(time.to_rfc3339_opts(SecondsFormat::Secs, true))
}

/// The inverse of [`format_time`]. Accepts only the form it writes (UTC,
/// whole seconds, `Z`), so a record has one spelling of each time.
pub fn parse_time(text: &str) -> Result<u64> {
    let time = DateTime::parse_from_rfc3339(text)?;
    let seconds = u64::try_from(time.timestamp())?;
    if format_time(seconds)? != text {
        return Err(anyhow!(
            "{text} is not in the form YYYY-MM-DDTHH:MM:SSZ (UTC, whole seconds)"
        ));
    }
    Ok(seconds)
}

// ---------------------------------------------------------------------------
// Verification
// ---------------------------------------------------------------------------

/// The key a verifier expects the log to be signed with.
#[derive(Clone)]
pub enum ExpectedKey {
    PublicKey(StrandSignaturePk),
    /// As [`fingerprint`] prints it; spaces, colons and case are ignored.
    Fingerprint(String),
}

impl ExpectedKey {
    /// A fingerprint (32 hex digits, grouped or not) or a base64 SPKI DER key.
    pub fn parse(text: &str) -> Result<ExpectedKey> {
        let digits = normalize_fingerprint(text);
        if digits.len() == FINGERPRINT_BYTES * 2 && digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(ExpectedKey::Fingerprint(text.to_string()));
        }
        Ok(ExpectedKey::PublicKey(
            StrandSignaturePk::from_der_b64_string(text.trim())
                .map_err(|error| anyhow!("not a fingerprint nor a base64 key: {error}"))?,
        ))
    }

    fn matches(&self, pk: &StrandSignaturePk) -> Result<bool> {
        Ok(match self {
            ExpectedKey::PublicKey(expected) => expected.to_der()? == pk.to_der()?,
            ExpectedKey::Fingerprint(expected) => {
                normalize_fingerprint(expected) == normalize_fingerprint(&fingerprint(pk)?)
            }
        })
    }
}

fn normalize_fingerprint(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(c, ' ' | ':' | '-'))
        .collect::<String>()
        .to_ascii_uppercase()
}

/// Which check of a seal failed, in the order they run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display)]
pub enum SealCheckStep {
    Format,
    Entries,
    Manifest,
    Message,
    SealHash,
    Statement,
    Time,
    CloseRequest,
    SystemSignature,
    SenderSignature,
    Key,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealCheckError {
    pub step: SealCheckStep,
    pub reason: String,
}

impl fmt::Display for SealCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.step, self.reason)
    }
}

impl std::error::Error for SealCheckError {}

fn fail<T>(step: SealCheckStep, reason: impl Into<String>) -> Result<T, SealCheckError> {
    Err(SealCheckError {
        step,
        reason: reason.into(),
    })
}

/// What a valid record proves, for the verifier's report.
#[derive(Debug)]
pub struct SealCheck {
    pub manifest: BallotBoxSealManifest,
    pub message: Message,
    pub seal_hash: Hash,
    pub system_key_fingerprint: String,
}

/// The rules every manifest the sealer writes follows: the format, the
/// time order, weights only on counted entries (at least 1) and no more
/// counted ballots than eligible voters (one counted entry per voter).
pub fn check_manifest(manifest: &BallotBoxSealManifest) -> Result<(), SealCheckError> {
    if manifest.format != SEAL_FORMAT_V1 {
        return fail(
            SealCheckStep::Format,
            format!("unknown format {}", manifest.format),
        );
    }
    if !(manifest.closed_at <= manifest.grace_deadline
        && manifest.grace_deadline <= manifest.sealed_at)
    {
        return fail(
            SealCheckStep::Time,
            "the times are out of order: closed, then the grace deadline, then sealed",
        );
    }
    for entry in &manifest.entries {
        let counted = entry.disposition == SealDisposition::Counted;
        if counted && entry.weight == 0 {
            return fail(
                SealCheckStep::Entries,
                format!("Ballot ID {} counts with weight 0", entry.ballot_id),
            );
        }
        if !counted && entry.weight != 0 {
            return fail(
                SealCheckStep::Entries,
                format!(
                    "Ballot ID {} does not count but has a weight",
                    entry.ballot_id
                ),
            );
        }
    }
    if manifest.ballots_counted() > manifest.eligible_voters {
        return fail(
            SealCheckStep::Entries,
            format!(
                "{} ballots counted but only {} eligible voters",
                manifest.ballots_counted(),
                manifest.eligible_voters
            ),
        );
    }
    Ok(())
}

/// Checks a signed `BallotBoxSealed` message against a manifest and its
/// exact bytes: the manifest's own rules ([`check_manifest`]), the hash, the
/// statement's event, election, area, counts and Close voting request, and
/// the seal time. Signatures are not checked here (see `Message::verify`).
/// Returns the seal hash.
pub fn check_statement(
    message: &Message,
    manifest: &BallotBoxSealManifest,
    manifest_bytes: &[u8],
) -> Result<Hash, SealCheckError> {
    check_manifest(manifest)?;
    let hash = seal_hash(manifest_bytes)
        .or_else(|error| fail(SealCheckStep::SealHash, error.to_string()))?;
    let StatementBody::BallotBoxSealed(election, area, signed_hash, in_box, counted, close_request) =
        &message.statement.body
    else {
        return fail(
            SealCheckStep::Statement,
            "the message is not a BallotBoxSealed statement",
        );
    };
    if !matches!(message.statement.head.kind, StatementType::BallotBoxSealed) {
        return fail(
            SealCheckStep::Statement,
            "the statement's kind is not BallotBoxSealed",
        );
    }
    if signed_hash.to_inner() != hash {
        return fail(
            SealCheckStep::SealHash,
            format!(
                "SHA-512 {} is not the hash in the signed statement ({})",
                hex::encode(hash),
                signed_hash.to_hex()
            ),
        );
    }
    let mut differs = Vec::new();
    if message.statement.head.event.0 != manifest.election_event_id {
        differs.push("election event");
    }
    if election.0.as_deref() != Some(manifest.election_id.as_str()) {
        differs.push("election");
    }
    if area.0 != manifest.area_id {
        differs.push("area");
    }
    if *in_box != manifest.ballots_in_box() {
        differs.push("ballots in the box");
    }
    if *counted != manifest.ballots_counted() {
        differs.push("ballots counted");
    }
    if !differs.is_empty() {
        return fail(
            SealCheckStep::Statement,
            format!(
                "the statement's {} differ from the manifest's",
                differs.join(", ")
            ),
        );
    }
    if message.statement.head.timestamp != manifest.sealed_at {
        return fail(
            SealCheckStep::Time,
            format!(
                "the signed statement says {}, the manifest {}",
                message.statement.head.timestamp, manifest.sealed_at
            ),
        );
    }
    if *close_request != manifest.close_request_id {
        return fail(
            SealCheckStep::CloseRequest,
            "the statement's Close voting request differs from the manifest's",
        );
    }
    Ok(hash)
}

/// Verifies a seal record offline: the manifest rebuilt from the
/// header and entries equals the record's bytes, its hash is the signed
/// one, the statement matches the manifest, both signatures hold and, when
/// one is expected, the key is the expected one.
pub fn verify_record(
    record: &SealRecord,
    expected_key: Option<&ExpectedKey>,
) -> Result<SealCheck, SealCheckError> {
    use SealCheckStep as Step;
    if record.format != SEAL_FORMAT_V1 {
        return fail(Step::Format, format!("unknown format {}", record.format));
    }

    let mut sorted = record.entries.clone();
    sorted.sort();
    if sorted != record.entries {
        return fail(Step::Entries, "the entries are not sorted by ballot hash");
    }
    let time =
        |text: &str| parse_time(text).or_else(|error| fail(Step::Manifest, error.to_string()));
    let rebuilt = BallotBoxSealManifest {
        format: record.format.clone(),
        tenant_id: record.tenant_id.clone(),
        election_event_id: record.election_event.id.clone(),
        election_id: record.election.id.clone(),
        area_id: record.area.id.clone(),
        closed_at: time(&record.closed_at)?,
        grace_deadline: time(&record.grace_deadline)?,
        sealed_at: time(&record.sealed_at)?,
        close_request_id: record
            .close_request
            .as_ref()
            .map(|request| request.id.clone()),
        eligible_voters: record.eligible_voters,
        entries: record.entries.clone(),
    };
    check_manifest(&rebuilt)?;
    let rebuilt_bytes = rebuilt
        .to_bytes()
        .or_else(|error| fail(Step::Manifest, error.to_string()))?;
    let record_bytes = general_purpose::STANDARD
        .decode(&record.manifest)
        .or_else(|error| {
            fail(
                Step::Manifest,
                format!("the manifest is not base64: {error}"),
            )
        })?;
    if rebuilt_bytes != record_bytes {
        return fail(
            Step::Manifest,
            "the manifest rebuilt from the header and entries differs from the record's",
        );
    }
    if record.ballots.in_the_box != rebuilt.ballots_in_box()
        || record.ballots.counted != rebuilt.ballots_counted()
    {
        return fail(
            Step::Manifest,
            "the record's counts differ from its entries",
        );
    }

    let message_bytes = general_purpose::STANDARD
        .decode(&record.message_b64)
        .or_else(|error| fail(Step::Message, format!("the message is not base64: {error}")))?;
    let message = Message::strand_deserialize(&message_bytes).or_else(|error| {
        fail(
            Step::Message,
            format!("the message does not decode: {error}"),
        )
    })?;
    let readable =
        serde_json::to_value(&message).or_else(|error| fail(Step::Message, error.to_string()))?;
    if readable != record.message {
        return fail(
            Step::Message,
            "the readable message differs from the signed bytes",
        );
    }
    if message.election_id.as_deref() != Some(rebuilt.election_id.as_str())
        || message.area_id.as_deref() != Some(rebuilt.area_id.as_str())
    {
        return fail(
            Step::Message,
            "the log row's election or area differs from the manifest's",
        );
    }

    let hash = check_statement(&message, &rebuilt, &record_bytes)?;
    if hex::encode(hash) != record.seal_hash.to_ascii_lowercase() {
        return fail(
            Step::SealHash,
            "the record's seal hash is not the manifest's SHA-512",
        );
    }

    let system_pk =
        StrandSignaturePk::from_der_b64_string(&record.system_public_key).or_else(|error| {
            fail(
                Step::SystemSignature,
                format!("the system key does not decode: {error}"),
            )
        })?;
    let statement_bytes = message
        .statement
        .strand_serialize()
        .or_else(|error| fail(Step::Message, error.to_string()))?;
    system_pk
        .verify(&message.system_signature, &statement_bytes)
        .or_else(|error| fail(Step::SystemSignature, error.to_string()))?;
    message
        .sender
        .pk
        .verify(&message.sender_signature, &statement_bytes)
        .or_else(|error| fail(Step::SenderSignature, error.to_string()))?;

    // The sealer signs with the system key as sender: a seal is a system
    // entry, so any other sender is a forgery.
    let key_der = system_pk
        .to_der()
        .or_else(|error| fail(Step::SystemSignature, error.to_string()))?;
    let sender_der = message
        .sender
        .pk
        .to_der()
        .or_else(|error| fail(Step::SenderSignature, error.to_string()))?;
    if sender_der != key_der {
        return fail(
            Step::SenderSignature,
            "the sender key is not the system key: not a system entry",
        );
    }
    let system_key_fingerprint =
        fingerprint(&system_pk).or_else(|error| fail(Step::Key, error.to_string()))?;
    if let Some(expected) = expected_key {
        let matches = expected
            .matches(&system_pk)
            .or_else(|error| fail(Step::Key, error.to_string()))?;
        if !matches {
            return fail(
                Step::Key,
                format!("signed by key {system_key_fingerprint}, not the expected key"),
            );
        }
    }

    Ok(SealCheck {
        manifest: rebuilt,
        message,
        seal_hash: hash,
        system_key_fingerprint,
    })
}

/// Serde for a 64-byte hash as lowercase hex.
mod hash_hex {
    use serde::{Deserialize, Deserializer, Serializer};
    use strand::hash::Hash;

    pub fn serialize<S: Serializer>(hash: &Hash, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&hex::encode(hash))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Hash, D::Error> {
        let text = String::deserialize(deserializer)?;
        hex::decode(&text)
            .map_err(serde::de::Error::custom)?
            .try_into()
            .map_err(|_| serde::de::Error::custom("a ballot hash is 64 bytes"))
    }
}

#[cfg(test)]
#[path = "../tests/support/seal_tests.rs"]
mod tests;
