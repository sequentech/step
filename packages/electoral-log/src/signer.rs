// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Who signs a log message. A message is signed by its sender and by the
//! log's system key, each over the statement's exact serialized bytes
//! (Ed25519). The keys are usually [`SigningKey`]s held in memory; an
//! installation may instead plug in a signer that keeps its key elsewhere.
//! Either way the signatures are the same Ed25519 signatures over the same
//! bytes, and verifiers don't change.

use std::sync::Arc;

use anyhow::Result;
use voting_crypto::signing::{Signature, SigningKey, VerificationKey};

/// Signs log messages with one Ed25519 key.
pub trait MessageSigner: Send + Sync {
    fn public_key(&self) -> Result<VerificationKey>;
    /// Signs exactly `bytes` (no hashing, no prefix).
    fn sign(&self, bytes: &[u8]) -> Result<Signature>;
}

impl MessageSigner for SigningKey {
    fn public_key(&self) -> Result<VerificationKey> {
        VerificationKey::from_sk(self)
    }

    fn sign(&self, bytes: &[u8]) -> Result<Signature> {
        SigningKey::sign(self, bytes)
    }
}

/// A shared signer.
pub type SharedSigner = Arc<dyn MessageSigner>;

/// A [`SigningKey`] as a shared signer.
pub fn software(key: SigningKey) -> SharedSigner {
    Arc::new(key)
}
