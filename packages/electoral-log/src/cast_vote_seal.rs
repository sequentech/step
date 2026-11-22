// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The stored form of a cast vote sealed to its election event's HSM key
//! (`hsm-seal-v1`), shared by the ballot box, its readers and the verifiers.
//!
//! The voter's ballot (the inner content, still encrypted to the election's
//! key) is sealed whole, and the stored `content` is this envelope as JSON.
//! A row's `format` is `hsm-seal-v1+<inner format>`, so plaintext and sealed
//! rows never need to be told apart by their content. The Ballot ID, the
//! cast-vote hash and the inner format travel in clear: they are the inner
//! ballot's hashes, which receipts and the electoral log already publish.
//!
//! The associated data binds an envelope to its row: a domain tag, then the
//! tenant, event, election, area, Ballot ID and inner format, each as a
//! big-endian `u32` length and its UTF-8 bytes. UUIDs are written in their
//! canonical lowercase hyphenated form whatever form they are given in.

use anyhow::{anyhow, ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The sealed format, as the HSM service names it.
pub const SEAL_FORMAT: &str = "hsm-seal-v1";
/// Separates [`SEAL_FORMAT`] from the inner format in a row's `format`.
const FORMAT_SEPARATOR: char = '+';
/// The domain tag that starts the associated data.
pub const AAD_DOMAIN: &[u8] = b"sequent-cast-vote-seal-v1";
/// The envelope version this module writes and reads.
pub const ENVELOPE_VERSION: u32 = 1;
/// Hex digits of a Ballot ID (SHA-512 truncated to 256 bits).
const BALLOT_ID_HEX: usize = 64;
/// Hex digits of a cast-vote hash (SHA-512).
const CAST_VOTE_HASH_HEX: usize = 128;

/// The `format` of a row sealed over a ballot of `inner` format.
pub fn sealed_format(inner: &str) -> String {
    format!("{SEAL_FORMAT}{FORMAT_SEPARATOR}{inner}")
}

/// The inner format of a sealed row's `format`; `None` for a plaintext row.
pub fn inner_format(format: &str) -> Option<&str> {
    format
        .strip_prefix(SEAL_FORMAT)
        .and_then(|rest| rest.strip_prefix(FORMAT_SEPARATOR))
        .filter(|inner| !inner.is_empty())
}

/// Whether a row's `format` is a sealed one.
pub fn is_sealed_format(format: &str) -> bool {
    inner_format(format).is_some()
}

/// Where a sealed ballot is stored: what its associated data binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealScope<'a> {
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub election_id: &'a str,
    pub area_id: &'a str,
    /// The inner ballot's Ballot ID.
    pub ballot_id: &'a str,
    pub inner_format: &'a str,
}

fn canonical(name: &str, id: &str) -> Result<String> {
    Ok(Uuid::parse_str(id)
        .with_context(|| format!("The {name} of a sealed ballot is not a UUID"))?
        .hyphenated()
        .to_string())
}

fn push_field(aad: &mut Vec<u8>, field: &[u8]) -> Result<()> {
    let length = u32::try_from(field.len()).context("An associated data field is too long")?;
    aad.extend_from_slice(&length.to_be_bytes());
    aad.extend_from_slice(field);
    Ok(())
}

/// The associated data of a sealed ballot stored in `scope`.
pub fn associated_data(scope: &SealScope<'_>) -> Result<Vec<u8>> {
    let mut aad = AAD_DOMAIN.to_vec();
    for field in [
        canonical("tenant", scope.tenant_id)?,
        canonical("election event", scope.election_event_id)?,
        canonical("election", scope.election_id)?,
        canonical("area", scope.area_id)?,
    ] {
        push_field(&mut aad, field.as_bytes())?;
    }
    push_field(&mut aad, scope.ballot_id.as_bytes())?;
    push_field(&mut aad, scope.inner_format.as_bytes())?;
    Ok(aad)
}

/// A stored sealed ballot: the `content` of a sealed row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedBallot {
    /// [`ENVELOPE_VERSION`].
    pub v: u32,
    /// The HSM key the ballot is sealed to: `cast-vote-seal/<tenant>/<event>`.
    pub key_id: String,
    pub key_version: u32,
    /// The inner ballot's format, as an unsealed row would store it.
    pub inner_format: String,
    /// The inner ballot's Ballot ID: lowercase hex of SHA-512 truncated to
    /// 256 bits, as the cast computes it.
    pub ballot_id: String,
    /// The inner ballot's cast-vote hash: lowercase hex of its full SHA-512,
    /// as the cast-vote record carries it.
    pub cast_vote_hash: String,
    /// Standard base64 of the `hsm-seal-v1` bytes.
    pub sealed: String,
}

impl SealedBallot {
    /// The checks that need no key: the version, the hashes' form, the
    /// Ballot ID being the cast-vote hash's first half, and the sealed bytes'
    /// encoding.
    pub fn check(&self) -> Result<()> {
        ensure!(
            self.v == ENVELOPE_VERSION,
            "Unknown sealed ballot version {}",
            self.v
        );
        ensure!(
            !self.key_id.is_empty() && self.key_version >= 1,
            "A sealed ballot names no key"
        );
        ensure!(
            !self.inner_format.is_empty() && !is_sealed_format(&self.inner_format),
            "A sealed ballot has no inner format"
        );
        let lower_hex = |text: &str, length: usize| {
            text.len() == length
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        ensure!(
            lower_hex(&self.ballot_id, BALLOT_ID_HEX),
            "A sealed ballot's Ballot ID is not a hash"
        );
        ensure!(
            lower_hex(&self.cast_vote_hash, CAST_VOTE_HASH_HEX)
                && self.cast_vote_hash.starts_with(&self.ballot_id),
            "A sealed ballot's cast-vote hash does not match its Ballot ID"
        );
        self.sealed_bytes()?;
        Ok(())
    }

    /// The `hsm-seal-v1` bytes.
    pub fn sealed_bytes(&self) -> Result<Vec<u8>> {
        STANDARD
            .decode(&self.sealed)
            .map_err(|_| anyhow!("A sealed ballot's bytes are not base64"))
    }

    /// The cast-vote hash's bytes.
    pub fn cast_vote_hash_bytes(&self) -> Result<Vec<u8>> {
        hex::decode(&self.cast_vote_hash)
            .map_err(|_| anyhow!("A sealed ballot's cast-vote hash is not hex"))
    }

    /// The stored `content`.
    pub fn to_content(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// A stored `content`, checked as [`SealedBallot::check`] does.
    pub fn from_content(content: &str) -> Result<Self> {
        let envelope: SealedBallot =
            serde_json::from_str(content).context("A stored ballot is not a sealed ballot")?;
        envelope.check()?;
        Ok(envelope)
    }

    /// A stored sealed row's `content` and `format`, and its `ballot_id`
    /// column: the envelope must agree with all three.
    pub fn from_row(content: &str, format: &str, ballot_id: &str) -> Result<Self> {
        let inner = inner_format(format)
            .ok_or_else(|| anyhow!("The stored ballot {ballot_id} is not sealed"))?;
        let envelope = Self::from_content(content)?;
        ensure!(
            envelope.inner_format == inner,
            "The sealed ballot {ballot_id} does not match its row's format"
        );
        ensure!(
            envelope.ballot_id == ballot_id,
            "The sealed ballot {ballot_id} does not match its row's Ballot ID"
        );
        Ok(envelope)
    }
}

/// The event key a cast vote is sealed to, as the event's electoral log pins
/// it in a signed `CastVoteSealKeyPublished` statement. A cast checks the
/// published key against it, so a key substituted afterwards is refused.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CastVoteSealKeyPin {
    pub tenant_id: String,
    pub election_event_id: String,
    pub key_id: String,
    pub key_version: u32,
    /// [`SEAL_FORMAT`].
    pub format: String,
    /// Lowercase hex of the uncompressed P-256 public point.
    pub public_key: String,
    /// Lowercase hex of SHA-256 of the public point.
    pub fingerprint: String,
}

/// The key id of an election event's key.
pub fn key_id(tenant_id: &str, election_event_id: &str) -> Result<String> {
    Ok(format!(
        "cast-vote-seal/{}/{}",
        canonical("tenant", tenant_id)?,
        canonical("election event", election_event_id)?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "6f1c2c0e-55d3-4e6b-9f0e-0c8b3a8f6d21";
    const EVENT: &str = "0e4a8a3b-2f37-4b4f-a6d3-5d2c9f1e7b40";
    const ELECTION: &str = "a3d9b7f2-1c64-4e0a-8b5e-9f7c2d1a6e83";
    const AREA: &str = "c8e1f4a7-6b2d-4f93-a0c5-3e9d8b7a1f62";

    fn scope<'a>(ballot_id: &'a str) -> SealScope<'a> {
        SealScope {
            tenant_id: TENANT,
            election_event_id: EVENT,
            election_id: ELECTION,
            area_id: AREA,
            ballot_id,
            inner_format: "hashable-ballot",
        }
    }

    fn envelope() -> SealedBallot {
        let hash = "ab".repeat(64);
        SealedBallot {
            v: ENVELOPE_VERSION,
            key_id: key_id(TENANT, EVENT).unwrap(),
            key_version: 1,
            inner_format: "hashable-ballot".into(),
            ballot_id: hash[..64].to_string(),
            cast_vote_hash: hash,
            sealed: STANDARD.encode([1u8; 94]),
        }
    }

    #[test]
    fn formats_name_the_seal_and_the_inner_format() {
        assert_eq!(
            sealed_format("hashable-multi-ballot"),
            "hsm-seal-v1+hashable-multi-ballot"
        );
        assert_eq!(
            inner_format("hsm-seal-v1+hashable-ballot"),
            Some("hashable-ballot")
        );
        for plaintext in [
            "hashable-ballot",
            "hsm-seal-v1",
            "hsm-seal-v1+",
            "hsm-seal-v2+x",
        ] {
            assert_eq!(inner_format(plaintext), None, "{plaintext}");
            assert!(!is_sealed_format(plaintext));
        }
    }

    #[test]
    fn associated_data_is_tagged_length_prefixed_and_canonical() {
        let ballot_id = "cd".repeat(32);
        let aad = associated_data(&scope(&ballot_id)).unwrap();
        let mut expected = b"sequent-cast-vote-seal-v1".to_vec();
        for field in [TENANT, EVENT, ELECTION, AREA, &ballot_id, "hashable-ballot"] {
            expected.extend((field.len() as u32).to_be_bytes());
            expected.extend(field.as_bytes());
        }
        assert_eq!(aad, expected);
        // Upper case or braced UUIDs bind the same row.
        let upper = ELECTION.to_uppercase();
        let mut other = scope(&ballot_id);
        other.election_id = &upper;
        assert_eq!(associated_data(&other).unwrap(), aad);
        // Every field changes it.
        let mut moved = scope(&ballot_id);
        moved.area_id = ELECTION;
        assert_ne!(associated_data(&moved).unwrap(), aad);
        let mut reformatted = scope(&ballot_id);
        reformatted.inner_format = "hashable-multi-ballot";
        assert_ne!(associated_data(&reformatted).unwrap(), aad);
        // No field can run into the next.
        let mut left = scope("ab");
        left.inner_format = "c";
        let mut right = scope("a");
        right.inner_format = "bc";
        assert_ne!(
            associated_data(&left).unwrap(),
            associated_data(&right).unwrap()
        );
        let mut invalid = scope(&ballot_id);
        invalid.tenant_id = "tenant";
        assert!(associated_data(&invalid).is_err());
    }

    #[test]
    fn envelopes_round_trip_and_agree_with_their_row() {
        let envelope = envelope();
        let content = envelope.to_content().unwrap();
        assert_eq!(SealedBallot::from_content(&content).unwrap(), envelope);
        assert_eq!(
            SealedBallot::from_row(&content, "hsm-seal-v1+hashable-ballot", &envelope.ballot_id)
                .unwrap(),
            envelope
        );
        assert!(SealedBallot::from_row(&content, "hashable-ballot", &envelope.ballot_id).is_err());
        assert!(SealedBallot::from_row(
            &content,
            "hsm-seal-v1+hashable-multi-ballot",
            &envelope.ballot_id
        )
        .is_err());
        assert!(SealedBallot::from_row(&content, "hsm-seal-v1+hashable-ballot", "00").is_err());
        assert_eq!(envelope.cast_vote_hash_bytes().unwrap(), vec![0xab; 64]);
        assert_eq!(envelope.sealed_bytes().unwrap(), vec![1u8; 94]);
    }

    #[test]
    fn malformed_envelopes_are_refused() {
        let cases: Vec<Box<dyn Fn(&mut SealedBallot)>> = vec![
            Box::new(|e| e.v = 2),
            Box::new(|e| e.key_id.clear()),
            Box::new(|e| e.key_version = 0),
            Box::new(|e| e.inner_format = "hsm-seal-v1+hashable-ballot".into()),
            Box::new(|e| e.inner_format.clear()),
            Box::new(|e| e.ballot_id = "AB".repeat(32)),
            Box::new(|e| e.cast_vote_hash = "cd".repeat(64)),
            Box::new(|e| e.cast_vote_hash.truncate(100)),
            Box::new(|e| e.sealed = "not base64!".into()),
        ];
        for (index, case) in cases.iter().enumerate() {
            let mut bad = envelope();
            case(&mut bad);
            assert!(bad.check().is_err(), "case {index}");
            assert!(
                SealedBallot::from_content(&bad.to_content().unwrap()).is_err(),
                "case {index}"
            );
        }
        // A plaintext ballot is not an envelope, and nothing unknown is accepted.
        assert!(SealedBallot::from_content(r#"{"contests":[]}"#).is_err());
        let mut value = serde_json::to_value(envelope()).unwrap();
        value["extra"] = serde_json::json!(1);
        assert!(SealedBallot::from_content(&value.to_string()).is_err());
    }

    #[test]
    fn the_key_pin_statement_is_appended_and_round_trips() {
        use crate::messages::statement::{StatementBody, StatementType};
        let pin = CastVoteSealKeyPin {
            tenant_id: TENANT.into(),
            election_event_id: EVENT.into(),
            key_id: key_id(TENANT, EVENT).unwrap(),
            key_version: 1,
            format: SEAL_FORMAT.into(),
            public_key: format!("04{}", "11".repeat(64)),
            fingerprint: "22".repeat(32),
        };
        let bytes = borsh::to_vec(&StatementBody::CastVoteSealKeyPublished(pin.clone())).unwrap();
        // After every existing body variant (PrivateBallotEvent is 50).
        assert_eq!(bytes[0], 51);
        let StatementBody::CastVoteSealKeyPublished(decoded) =
            borsh::from_slice::<StatementBody>(&bytes).unwrap()
        else {
            panic!("not a key pin");
        };
        assert_eq!(decoded, pin);
        assert_eq!(
            StatementType::CastVoteSealKeyPublished.to_string(),
            "CastVoteSealKeyPublished"
        );
    }

    #[test]
    fn key_ids_are_canonical() {
        assert_eq!(
            key_id(&TENANT.to_uppercase(), EVENT).unwrap(),
            format!("cast-vote-seal/{TENANT}/{EVENT}")
        );
        assert!(key_id("x", EVENT).is_err());
    }
}
