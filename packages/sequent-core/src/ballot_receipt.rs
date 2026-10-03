// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the ballot box signs when it receives a ballot and when that ballot is
//! cast, and the Ballot ID that follows from the first.
//!
//! The ballot box signs only what it stores, so a Received or a Cast receipt
//! signature that verifies against the published ballot box key is proof of
//! storage. The Ballot ID is a hash of the voter-signed ballot and of the
//! Received signature: it cannot be known before the ballot box has answered.
//! The voter casts by signing that Ballot ID with the key that signed the
//! ballot.

use crate::ballot::BallotBoxKey;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use strand::hash::{hash_sha256, hash_to_array};
use strand::signature::{
    StrandSignature, StrandSignaturePk, StrandSignatureSk,
};

const RECEIVED_STATEMENT_DOMAIN: &str = "step/ballot-received/v1";
const BALLOT_ID_DOMAIN: &str = "step/ballot-id/v1";
const CAST_STATEMENT_DOMAIN: &str = "step/ballot-cast/v1";
const CAST_RECEIPT_DOMAIN: &str = "step/ballot-cast-receipt/v1";

const KEY_ID_BYTES: usize = 8;
const BALLOT_ID_BYTES: usize = 5;
const BALLOT_ID_CHARACTERS: usize = 8;
const BALLOT_ID_GROUP: usize = 4;
const BALLOT_ID_SEPARATOR: char = '-';
/// Crockford's base32: no I, L, O or U.
const BALLOT_ID_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

quick_error! {
    #[derive(Debug, PartialEq, Eq)]
    pub enum BallotReceiptError {
        Malformed(message: String) {
            display("Malformed ballot receipt: {}", message)
        }
        UnknownKey {
            display("The receipt was not signed with the published ballot box key")
        }
        InvalidSignature {
            display("The ballot box signature does not verify")
        }
        BallotIdMismatch {
            display("The Ballot ID does not follow from the receipt")
        }
        InvalidCastSignature {
            display("The voter's cast signature does not verify")
        }
        VoterKeyNotKept {
            display("The key that signed the ballot is no longer in memory")
        }
    }
}

/// What the ballot box states about a ballot it has stored.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ReceivedStatement {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    /// The hash of the encrypted ballot, as `encrypt::hash_ballot` gives it.
    pub ballot_hash: String,
    pub voter_signing_pk: String,
    pub voter_ballot_signature: String,
    /// UTC, RFC 3339 with milliseconds: 2028-05-08T03:00:00.000Z.
    pub received_at: String,
    pub key_id: String,
}

/// A Received statement with the ballot box's signature over it.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ReceivedBallot {
    #[serde(flatten)]
    pub statement: ReceivedStatement,
    pub received_signature: String,
    pub ballot_id: String,
}

fn malformed<E: std::fmt::Display>(
    field: &'static str,
) -> impl FnOnce(E) -> BallotReceiptError {
    move |error| BallotReceiptError::Malformed(format!("{field}: {error}"))
}

/// Length-prefixed like `ballot::get_ballot_bytes_for_signing`, so that no two
/// lists of fields share their bytes.
fn extend_with_field(bytes: &mut Vec<u8>, field: &[u8]) {
    bytes.extend_from_slice(&(field.len() as u64).to_le_bytes());
    bytes.extend_from_slice(field);
}

fn voter_key_bytes(
    statement: &ReceivedStatement,
) -> Result<Vec<u8>, BallotReceiptError> {
    StrandSignaturePk::from_der_b64_string(&statement.voter_signing_pk)
        .and_then(|key| key.to_der())
        .map_err(malformed("voter_signing_pk"))
}

fn voter_signature_bytes(
    statement: &ReceivedStatement,
) -> Result<[u8; 64], BallotReceiptError> {
    StrandSignature::from_b64_string(&statement.voter_ballot_signature)
        .map(|signature| signature.to_bytes())
        .map_err(malformed("voter_ballot_signature"))
}

impl ReceivedStatement {
    pub fn bytes_for_signing(&self) -> Result<Vec<u8>, BallotReceiptError> {
        let mut bytes = vec![];
        extend_with_field(&mut bytes, RECEIVED_STATEMENT_DOMAIN.as_bytes());
        extend_with_field(&mut bytes, self.tenant_id.as_bytes());
        extend_with_field(&mut bytes, self.election_event_id.as_bytes());
        extend_with_field(&mut bytes, self.election_id.as_bytes());
        extend_with_field(&mut bytes, self.ballot_hash.as_bytes());
        extend_with_field(&mut bytes, &voter_key_bytes(self)?);
        extend_with_field(&mut bytes, &voter_signature_bytes(self)?);
        extend_with_field(&mut bytes, self.received_at.as_bytes());
        extend_with_field(&mut bytes, self.key_id.as_bytes());
        Ok(bytes)
    }
}

/// The published form of a ballot box key: the first 8 bytes of SHA-256 of
/// its DER encoding name it.
pub fn ballot_box_key(
    public_key: &StrandSignaturePk,
) -> Result<BallotBoxKey, BallotReceiptError> {
    public_key
        .to_der()
        .and_then(|der| hash_sha256(&der))
        .and_then(|digest| {
            let key_id: Vec<u8> =
                digest.into_iter().take(KEY_ID_BYTES).collect();
            public_key
                .to_der_b64_string()
                .map(|public_key| BallotBoxKey {
                    key_id: hex::encode(key_id),
                    public_key,
                })
        })
        .map_err(malformed("public_key"))
}

/// Writes 40 bits as two groups of four characters: FTBE-MHRX.
pub fn format_ballot_id(bytes: &[u8; BALLOT_ID_BYTES]) -> String {
    let bits = bytes
        .iter()
        .fold(0u64, |bits, byte| (bits << 8) | u64::from(*byte));
    let mut id = String::with_capacity(BALLOT_ID_CHARACTERS + 1);
    for position in 0..BALLOT_ID_CHARACTERS {
        if position == BALLOT_ID_GROUP {
            id.push(BALLOT_ID_SEPARATOR);
        }
        let shift = 5 * (BALLOT_ID_CHARACTERS - 1 - position);
        let index = ((bits >> shift) & 0x1f) as usize;
        id.push(char::from(BALLOT_ID_ALPHABET[index]));
    }
    id
}

/// Reads a typed Ballot ID: any case, with or without the separator, O as 0,
/// I and L as 1. `None` when it cannot be a Ballot ID.
pub fn normalize_ballot_id(typed: &str) -> Option<String> {
    let characters = typed
        .chars()
        .filter(|character| {
            *character != BALLOT_ID_SEPARATOR && !character.is_whitespace()
        })
        .map(|character| match character.to_ascii_uppercase() {
            'O' => Some('0'),
            'I' | 'L' => Some('1'),
            upper
                if upper.is_ascii()
                    && BALLOT_ID_ALPHABET.contains(&(upper as u8)) =>
            {
                Some(upper)
            }
            _ => None,
        })
        .collect::<Option<Vec<char>>>()?;
    if characters.len() != BALLOT_ID_CHARACTERS {
        return None;
    }

    let (first, second) = characters.split_at(BALLOT_ID_GROUP);
    Some(
        first
            .iter()
            .chain(std::iter::once(&BALLOT_ID_SEPARATOR))
            .chain(second)
            .collect(),
    )
}

/// The first 40 bits of SHA-512 over the voter-signed ballot and the ballot
/// box's signature.
pub fn ballot_id(
    statement: &ReceivedStatement,
    received_signature: &str,
) -> Result<String, BallotReceiptError> {
    let received_signature =
        StrandSignature::from_b64_string(received_signature)
            .map_err(malformed("received_signature"))?;

    let mut bytes = vec![];
    extend_with_field(&mut bytes, BALLOT_ID_DOMAIN.as_bytes());
    extend_with_field(&mut bytes, statement.ballot_hash.as_bytes());
    extend_with_field(&mut bytes, &voter_key_bytes(statement)?);
    extend_with_field(&mut bytes, &voter_signature_bytes(statement)?);
    extend_with_field(&mut bytes, statement.received_at.as_bytes());
    extend_with_field(&mut bytes, statement.key_id.as_bytes());
    extend_with_field(&mut bytes, &received_signature.to_bytes());

    let digest = hash_to_array(&bytes).map_err(malformed("digest"))?;
    let mut prefix = [0u8; BALLOT_ID_BYTES];
    prefix.copy_from_slice(&digest[..BALLOT_ID_BYTES]);
    Ok(format_ballot_id(&prefix))
}

/// Signs the statement with the ballot box key. Callers sign only what they
/// have stored.
pub fn sign_received_ballot(
    ballot_box_sk: &StrandSignatureSk,
    statement: ReceivedStatement,
) -> Result<ReceivedBallot, BallotReceiptError> {
    let received_signature = ballot_box_sk
        .sign(&statement.bytes_for_signing()?)
        .and_then(|signature| signature.to_b64_string())
        .map_err(malformed("received_signature"))?;
    let ballot_id = ballot_id(&statement, &received_signature)?;

    Ok(ReceivedBallot {
        statement,
        received_signature,
        ballot_id,
    })
}

/// Checks that the published ballot box key signed this receipt and that its
/// Ballot ID follows from it. Returns the Ballot ID.
pub fn verify_received_ballot(
    key: &BallotBoxKey,
    received: &ReceivedBallot,
) -> Result<String, BallotReceiptError> {
    let public_key = signing_public_key(key, &received.statement.key_id)?;
    let signature =
        StrandSignature::from_b64_string(&received.received_signature)
            .map_err(malformed("received_signature"))?;
    public_key
        .verify(&signature, &received.statement.bytes_for_signing()?)
        .map_err(|_| BallotReceiptError::InvalidSignature)?;

    let ballot_id =
        ballot_id(&received.statement, &received.received_signature)?;
    if ballot_id != received.ballot_id {
        return Err(BallotReceiptError::BallotIdMismatch);
    }
    Ok(ballot_id)
}

/// The published ballot box key, if it is the one a receipt names.
fn signing_public_key(
    key: &BallotBoxKey,
    key_id: &str,
) -> Result<StrandSignaturePk, BallotReceiptError> {
    let public_key = StrandSignaturePk::from_der_b64_string(&key.public_key)
        .map_err(malformed("public_key"))?;
    if ballot_box_key(&public_key)?.key_id != key.key_id || key_id != key.key_id
    {
        return Err(BallotReceiptError::UnknownKey);
    }
    Ok(public_key)
}

/// What the voter signs to cast a ballot the ballot box has received.
fn cast_statement_bytes(election_id: &str, ballot_id: &str) -> Vec<u8> {
    let mut bytes = vec![];
    extend_with_field(&mut bytes, CAST_STATEMENT_DOMAIN.as_bytes());
    extend_with_field(&mut bytes, election_id.as_bytes());
    extend_with_field(&mut bytes, ballot_id.as_bytes());
    bytes
}

/// Signs "cast this Ballot ID" with the key that signed the ballot.
pub fn sign_cast_statement(
    voter_sk: &StrandSignatureSk,
    election_id: &str,
    ballot_id: &str,
) -> Result<String, BallotReceiptError> {
    voter_sk
        .sign(&cast_statement_bytes(election_id, ballot_id))
        .and_then(|signature| signature.to_b64_string())
        .map_err(malformed("cast_signature"))
}

/// Checks that the key that signed the received ballot asks to cast it.
pub fn verify_cast_signature(
    voter_signing_pk: &str,
    election_id: &str,
    ballot_id: &str,
    cast_signature: &str,
) -> Result<(), BallotReceiptError> {
    let public_key = StrandSignaturePk::from_der_b64_string(voter_signing_pk)
        .map_err(malformed("voter_signing_pk"))?;
    let signature = StrandSignature::from_b64_string(cast_signature)
        .map_err(malformed("cast_signature"))?;
    public_key
        .verify(&signature, &cast_statement_bytes(election_id, ballot_id))
        .map_err(|_| BallotReceiptError::InvalidCastSignature)
}

/// What the ballot box states about a ballot it has stored as cast.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct CastReceiptStatement {
    pub election_event_id: String,
    pub election_id: String,
    pub ballot_id: String,
    /// As in the ballot's `ReceivedStatement`.
    pub received_at: String,
    /// UTC, RFC 3339 with milliseconds.
    pub cast_at: String,
    pub key_id: String,
    /// The voter's signature over the Cast statement.
    pub cast_signature: String,
}

/// A Cast receipt statement with the ballot box's signature over it.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct CastReceipt {
    #[serde(flatten)]
    pub statement: CastReceiptStatement,
    pub cast_receipt_signature: String,
}

impl CastReceiptStatement {
    pub fn bytes_for_signing(&self) -> Result<Vec<u8>, BallotReceiptError> {
        let cast_signature =
            StrandSignature::from_b64_string(&self.cast_signature)
                .map_err(malformed("cast_signature"))?;
        let cast_signature_digest = hash_sha256(&cast_signature.to_bytes())
            .map_err(malformed("cast_signature"))?;

        let mut bytes = vec![];
        extend_with_field(&mut bytes, CAST_RECEIPT_DOMAIN.as_bytes());
        extend_with_field(&mut bytes, self.election_event_id.as_bytes());
        extend_with_field(&mut bytes, self.election_id.as_bytes());
        extend_with_field(&mut bytes, self.ballot_id.as_bytes());
        extend_with_field(&mut bytes, self.received_at.as_bytes());
        extend_with_field(&mut bytes, self.cast_at.as_bytes());
        extend_with_field(&mut bytes, self.key_id.as_bytes());
        extend_with_field(&mut bytes, &cast_signature_digest);
        Ok(bytes)
    }
}

/// Signs the statement with the ballot box key. Callers sign only a cast they
/// store in the same transaction.
pub fn sign_cast_receipt(
    ballot_box_sk: &StrandSignatureSk,
    statement: CastReceiptStatement,
) -> Result<CastReceipt, BallotReceiptError> {
    let cast_receipt_signature = ballot_box_sk
        .sign(&statement.bytes_for_signing()?)
        .and_then(|signature| signature.to_b64_string())
        .map_err(malformed("cast_receipt_signature"))?;

    Ok(CastReceipt {
        statement,
        cast_receipt_signature,
    })
}

/// Checks that the published ballot box key signed this cast receipt.
pub fn verify_cast_receipt(
    key: &BallotBoxKey,
    receipt: &CastReceipt,
) -> Result<(), BallotReceiptError> {
    let public_key = signing_public_key(key, &receipt.statement.key_id)?;
    let signature =
        StrandSignature::from_b64_string(&receipt.cast_receipt_signature)
            .map_err(malformed("cast_receipt_signature"))?;
    public_key
        .verify(&signature, &receipt.statement.bytes_for_signing()?)
        .map_err(|_| BallotReceiptError::InvalidSignature)
}

thread_local! {
    /// The single-use keys that signed the ballots under review, by election.
    /// They live only in this memory: a reload loses them, and the ballot is
    /// then signed and sent again.
    static VOTER_SIGNING_KEYS: RefCell<HashMap<String, StrandSignatureSk>> =
        RefCell::new(HashMap::new());
}

/// Keeps the key that signed an election's ballot until that ballot is cast.
/// A new ballot for the election replaces the key of the one before.
pub fn keep_voter_signing_key(election_id: &str, voter_sk: StrandSignatureSk) {
    VOTER_SIGNING_KEYS.with(|keys| {
        keys.borrow_mut().insert(election_id.to_string(), voter_sk);
    });
}

pub fn forget_voter_signing_key(election_id: &str) {
    VOTER_SIGNING_KEYS.with(|keys| {
        keys.borrow_mut().remove(election_id);
    });
}

/// Signs the Cast statement with the key kept for the election, if it is the
/// one that signed the received ballot.
pub fn sign_cast_statement_with_kept_key(
    election_id: &str,
    voter_signing_pk: &str,
    ballot_id: &str,
) -> Result<String, BallotReceiptError> {
    VOTER_SIGNING_KEYS.with(|keys| {
        let keys = keys.borrow();
        let voter_sk = keys
            .get(election_id)
            .ok_or(BallotReceiptError::VoterKeyNotKept)?;
        let kept_pk = StrandSignaturePk::from_sk(voter_sk)
            .and_then(|key| key.to_der_b64_string())
            .map_err(malformed("voter_signing_pk"))?;
        if kept_pk != voter_signing_pk {
            return Err(BallotReceiptError::VoterKeyNotKept);
        }
        sign_cast_statement(voter_sk, election_id, ballot_id)
    })
}
