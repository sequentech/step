// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The electoral log's signature scheme (v3): an Ed25519 signature over a
//! short, domain-separated digest of an object's canonical bytes,
//!
//! ```text
//! signed = domain || 0x00 || SHA-256(canonical bytes)
//! ```
//!
//! The canonical bytes themselves are unchanged (a statement's record bytes,
//! a checkpoint's tuple); only what the key signs is. The signed message is
//! the same few dozen bytes whatever the object's size, so every signer
//! (a key in memory or one held in a device) signs the same way. This is
//! plain Ed25519 over that message, not Ed25519ph.
//!
//! The digest is SHA-256: it gives the same collision resistance (128-bit)
//! as Ed25519 itself and as the log's SHA-256 Merkle tree, which already
//! commits to these records. Other SHA-512 uses (such as a seal's manifest
//! hash) are separate and unchanged.
//!
//! Each kind of object has its own domain, so a signature for one can never
//! be taken for another. Signatures made by earlier versions (over the raw
//! bytes) do not verify here: there is no fallback to them.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use voting_crypto::signing::{Signature, VerificationKey};

use crate::signer::MessageSigner;

/// The signature scheme this module implements: a log message records it
/// (`Message::signature_version`), and a verifier refuses any other.
pub const SIGNATURE_VERSION: u8 = 3;

/// The domain of every log statement's signatures (sender and system).
pub const STATEMENT_DOMAIN: &str = "sequent-electoral-log/statement-signature/v3";
/// The domain of a published checkpoint's signature.
pub const CHECKPOINT_DOMAIN: &str = "sequent-electoral-log/checkpoint-signature/v3";

/// The kinds of object the log signs, each with its own domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    /// A log statement's record bytes (sender and system signatures).
    Statement,
    /// A published checkpoint's canonical bytes.
    Checkpoint,
}

impl Domain {
    pub const ALL: [Domain; 2] = [Domain::Statement, Domain::Checkpoint];

    pub fn as_str(self) -> &'static str {
        match self {
            Domain::Statement => STATEMENT_DOMAIN,
            Domain::Checkpoint => CHECKPOINT_DOMAIN,
        }
    }

    /// The domain named by `text`, if it is one of this scheme's.
    pub fn parse(text: &str) -> Option<Domain> {
        Domain::ALL
            .into_iter()
            .find(|domain| domain.as_str() == text)
    }
}

/// What the key signs for `canonical` under `domain`.
pub fn signed_message(domain: Domain, canonical: &[u8]) -> Vec<u8> {
    let domain = domain.as_str();
    let mut message = Vec::with_capacity(domain.len() + 1 + 32);
    message.extend_from_slice(domain.as_bytes());
    message.push(0);
    message.extend_from_slice(&Sha256::digest(canonical));
    message
}

/// Signs `canonical` under `domain`.
pub fn sign(signer: &dyn MessageSigner, domain: Domain, canonical: &[u8]) -> Result<Signature> {
    signer.sign(&signed_message(domain, canonical))
}

/// Verifies a signature of `canonical` under `domain`.
pub fn verify(
    key: &VerificationKey,
    signature: &Signature,
    domain: Domain,
    canonical: &[u8],
) -> Result<()> {
    key.verify(signature, &signed_message(domain, canonical))
        .with_context(|| format!("the signature does not verify ({})", domain.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use voting_crypto::signing::SigningKey;

    #[test]
    fn the_signed_message_is_short_and_fixed_by_domain_and_bytes() {
        let small = signed_message(Domain::Statement, b"x");
        let large = signed_message(Domain::Statement, &vec![7u8; 1 << 20]);
        assert_eq!(small.len(), large.len(), "independent of the object's size");
        assert_eq!(small.len(), STATEMENT_DOMAIN.len() + 33);
        assert!(small.len() < 128);
        // Vector: the layout is domain, a zero byte, then SHA-256.
        let mut expected = STATEMENT_DOMAIN.as_bytes().to_vec();
        expected.push(0);
        expected.extend_from_slice(
            &hex::decode("2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881")
                .unwrap(),
        );
        assert_eq!(small, expected);
    }

    #[test]
    fn every_domain_names_the_recorded_signature_version() {
        let suffix = format!("/v{SIGNATURE_VERSION}");
        for domain in Domain::ALL {
            assert!(domain.as_str().ends_with(&suffix), "{domain:?}");
            assert_eq!(Domain::parse(domain.as_str()), Some(domain));
        }
        assert_ne!(STATEMENT_DOMAIN, CHECKPOINT_DOMAIN);
        assert!(!STATEMENT_DOMAIN.starts_with(CHECKPOINT_DOMAIN));
        assert!(!CHECKPOINT_DOMAIN.starts_with(STATEMENT_DOMAIN));
        assert_eq!(
            Domain::parse("sequent-electoral-log/statement-signature/v2"),
            None
        );
    }

    #[test]
    fn a_signature_holds_for_its_domain_and_bytes_only() {
        let key = SigningKey::generate().unwrap();
        let public = VerificationKey::from_sk(&key).unwrap();
        let signature = sign(&key, Domain::Statement, b"statement").unwrap();
        verify(&public, &signature, Domain::Statement, b"statement").unwrap();
        assert!(
            verify(&public, &signature, Domain::Checkpoint, b"statement").is_err(),
            "another domain"
        );
        assert!(
            verify(&public, &signature, Domain::Statement, b"statemenT").is_err(),
            "other bytes"
        );
        // No fallback: a signature over the raw bytes (the earlier scheme) is refused.
        let raw = key.sign(b"statement").unwrap();
        assert!(verify(&public, &raw, Domain::Statement, b"statement").is_err());
    }
}
