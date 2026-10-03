// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the ballot box signs when it receives a ballot, and the Ballot ID that
//! follows from it.
//!
//! The ballot box signs only after it has stored the ballot, so a Received
//! signature that verifies against the published ballot box key is proof of
//! storage. The Ballot ID is a hash of the voter-signed ballot and of that
//! signature: it cannot be known before the ballot box has answered.

use crate::ballot::BallotBoxKey;
use serde::{Deserialize, Serialize};
use strand::hash::{hash_sha256, hash_to_array};
use strand::signature::{
    StrandSignature, StrandSignaturePk, StrandSignatureSk,
};

const RECEIVED_STATEMENT_DOMAIN: &str = "step/ballot-received/v1";
const BALLOT_ID_DOMAIN: &str = "step/ballot-id/v1";

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
    let public_key = StrandSignaturePk::from_der_b64_string(&key.public_key)
        .map_err(malformed("public_key"))?;
    if ballot_box_key(&public_key)?.key_id != key.key_id
        || received.statement.key_id != key.key_id
    {
        return Err(BallotReceiptError::UnknownKey);
    }

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
