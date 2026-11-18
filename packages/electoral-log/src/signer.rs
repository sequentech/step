// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Who signs a log message. A message is signed by its sender and by the
//! log's system key, each over the statement's exact serialized bytes
//! (Ed25519). The keys are usually [`SigningKey`]s held in memory; an
//! installation may instead plug in a signer that keeps its key elsewhere.
//! Either way the signatures are the same Ed25519 signatures over the same
//! bytes, and verifiers don't change.
//!
//! A signer that keeps its key elsewhere may be unable to sign for a while
//! (its key service is unreachable or overloaded). It then fails with a
//! [`SignerUnavailable`] in its error's chain: the message is not wrong, the
//! signing should be tried again later. Callers tell the two apart with
//! [`is_signer_unavailable`], which also sees the type under any
//! `.context(...)` added on the way up; any other signing error is final.
//! Code between a signer and such a caller must keep the error's chain
//! (`?`, `.context(...)`), not rebuild it from its text.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use voting_crypto::signing::{Signature, SigningKey, VerificationKey};

/// A signer that holds its key elsewhere cannot sign right now; signing
/// again later may succeed. Any other signing error is final.
#[derive(Debug)]
pub struct SignerUnavailable {
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl SignerUnavailable {
    /// Unavailable, for the reason `message` gives.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// Unavailable because of `source`, which is kept as the error's source.
    pub fn because(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

impl fmt::Display for SignerUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SignerUnavailable {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}

/// Whether `error` is, or was caused by, a [`SignerUnavailable`]: a signing
/// to try again later rather than a failure of the message. It is found
/// anywhere in the chain of causes, and as a `.context(...)` too.
pub fn is_signer_unavailable(error: &anyhow::Error) -> bool {
    error.downcast_ref::<SignerUnavailable>().is_some()
        || error.chain().any(|cause| cause.is::<SignerUnavailable>())
}

/// Signs log messages with one Ed25519 key.
pub trait MessageSigner: Send + Sync {
    fn public_key(&self) -> Result<VerificationKey>;
    /// Signs exactly `bytes` (no hashing, no prefix).
    fn sign(&self, bytes: &[u8]) -> Result<Signature>;
    /// One signature per message, in order, each exactly as [`sign`] gives.
    /// A signer whose key is held elsewhere may sign them in one round trip.
    ///
    /// [`sign`]: MessageSigner::sign
    fn sign_many(&self, messages: &[&[u8]]) -> Result<Vec<Signature>> {
        messages.iter().map(|bytes| self.sign(bytes)).collect()
    }
    /// Whether [`sign_many`] saves work over signing one by one (a signer
    /// whose key is held elsewhere). Only such a signer takes [`batch`]'s
    /// two passes; others build once, exactly as without it.
    ///
    /// [`sign_many`]: MessageSigner::sign_many
    fn batches(&self) -> bool {
        false
    }
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

/// Builds several log entries with one [`MessageSigner::sign_many`] call,
/// for a signer that [batches](MessageSigner::batches); any other signer's
/// entries are built once, as without this.
///
/// `build` is called twice: first with a signer that only records the bytes
/// to sign and answers with placeholder signatures (that result is
/// dropped), then, once all are signed in one call, with a signer that
/// hands back those signatures for exactly those bytes. So `build` must:
///
/// - be pure: no writes, sends or other effects (it runs twice, and its
///   first run's output is thrown away);
/// - not check the signatures it is given (the first run's are
///   placeholders, made by a throwaway key);
/// - for speed only, make the same bytes both times (no randomness).
///   Statements made by `build` carry one timestamp, the batch's (decided:
///   the batch's entries are made in the same second, at its start). Bytes
///   not recorded the first time are signed one by one, so a result never
///   depends on this, only the number of signing calls does.
pub fn batch<T>(signer: &SharedSigner, build: impl Fn(&SharedSigner) -> Result<T>) -> Result<T> {
    // A signer that doesn't batch (a key in memory) builds once, with each
    // statement's own time: nothing changes for it.
    if !signer.batches() {
        return build(signer);
    }
    // Statements carry the time they were made: both passes see the same.
    crate::with_fixed_timestamp(|| batch_now(signer, &build))
}

fn batch_now<T>(signer: &SharedSigner, build: &impl Fn(&SharedSigner) -> Result<T>) -> Result<T> {
    let public_key = signer.public_key()?;
    let recorder = Arc::new(Recorder {
        public_key: public_key.clone(),
        placeholder: SigningKey::generate()?,
        bytes: Mutex::new(Vec::new()),
    });
    let recording: SharedSigner = recorder.clone();
    drop(build(&recording)?);
    drop(recording);
    let recorded = std::mem::take(
        &mut *recorder
            .bytes
            .lock()
            .map_err(|_| anyhow!("the batch recorder's lock is poisoned"))?,
    );
    let mut unique: Vec<Vec<u8>> = Vec::with_capacity(recorded.len());
    let mut seen = std::collections::HashSet::new();
    for bytes in recorded {
        if seen.insert(bytes.clone()) {
            unique.push(bytes);
        }
    }
    let views: Vec<&[u8]> = unique.iter().map(Vec::as_slice).collect();
    let signatures = if views.is_empty() {
        Vec::new()
    } else {
        signer.sign_many(&views)?
    };
    if signatures.len() != unique.len() {
        return Err(anyhow!(
            "the signer gave {} signatures for {} messages",
            signatures.len(),
            unique.len()
        ));
    }
    let replay: SharedSigner = Arc::new(Replay {
        signer: signer.clone(),
        public_key,
        signatures: unique.into_iter().zip(signatures).collect(),
    });
    build(&replay)
}

/// Records what would be signed; its signatures are placeholders, made by a
/// throwaway key, and never leave [`batch`].
struct Recorder {
    public_key: VerificationKey,
    placeholder: SigningKey,
    bytes: Mutex<Vec<Vec<u8>>>,
}

impl MessageSigner for Recorder {
    fn public_key(&self) -> Result<VerificationKey> {
        Ok(self.public_key.clone())
    }
    fn sign(&self, bytes: &[u8]) -> Result<Signature> {
        self.bytes
            .lock()
            .map_err(|_| anyhow!("the batch recorder's lock is poisoned"))?
            .push(bytes.to_vec());
        self.placeholder.sign(bytes)
    }
}

/// Hands back the signatures made for exactly these bytes.
struct Replay {
    signer: SharedSigner,
    public_key: VerificationKey,
    signatures: HashMap<Vec<u8>, Signature>,
}

impl MessageSigner for Replay {
    fn public_key(&self) -> Result<VerificationKey> {
        Ok(self.public_key.clone())
    }
    fn sign(&self, bytes: &[u8]) -> Result<Signature> {
        match self.signatures.get(bytes) {
            Some(signature) => Ok(signature.clone()),
            None => self.signer.sign(bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A signer that counts its calls.
    struct Counting {
        key: SigningKey,
        singles: AtomicUsize,
        batches: AtomicUsize,
    }

    impl MessageSigner for Counting {
        fn public_key(&self) -> Result<VerificationKey> {
            VerificationKey::from_sk(&self.key)
        }
        fn sign(&self, bytes: &[u8]) -> Result<Signature> {
            self.singles.fetch_add(1, Ordering::SeqCst);
            self.key.sign(bytes)
        }
        fn sign_many(&self, messages: &[&[u8]]) -> Result<Vec<Signature>> {
            self.batches.fetch_add(1, Ordering::SeqCst);
            messages.iter().map(|m| self.key.sign(m)).collect()
        }
        fn batches(&self) -> bool {
            true
        }
    }

    fn counting() -> (Arc<Counting>, SharedSigner) {
        let counting = Arc::new(Counting {
            key: SigningKey::generate().unwrap(),
            singles: AtomicUsize::new(0),
            batches: AtomicUsize::new(0),
        });
        let shared: SharedSigner = counting.clone();
        (counting, shared)
    }

    #[test]
    fn a_batch_signs_once_and_gives_the_same_signatures() {
        let (counting, signer) = counting();
        let messages: Vec<Vec<u8>> = (0..5u8).map(|i| vec![i; 10]).collect();
        let signed = batch(&signer, |s| {
            messages
                .iter()
                .map(|m| s.sign(m))
                .collect::<Result<Vec<_>>>()
        })
        .unwrap();
        assert_eq!(counting.batches.load(Ordering::SeqCst), 1);
        assert_eq!(counting.singles.load(Ordering::SeqCst), 0);
        for (message, signature) in messages.iter().zip(&signed) {
            assert_eq!(
                signature,
                &counting.key.sign(message).unwrap(),
                "byte-identical"
            );
            signer
                .public_key()
                .unwrap()
                .verify(signature, message)
                .unwrap();
        }
    }

    #[test]
    fn a_builder_that_is_not_repeatable_is_still_signed_correctly() {
        let (counting, signer) = counting();
        let calls = AtomicUsize::new(0);
        let signed = batch(&signer, |s| {
            // Different bytes on the second call (as with a fresh timestamp).
            let n = calls.fetch_add(1, Ordering::SeqCst) as u8;
            Ok((vec![n, 1], s.sign(&[n, 1])?))
        })
        .unwrap();
        let (message, signature) = signed;
        assert_eq!(message, vec![1, 1]);
        signer
            .public_key()
            .unwrap()
            .verify(&signature, &message)
            .unwrap();
        assert_eq!(counting.singles.load(Ordering::SeqCst), 1, "signed singly");
    }

    #[test]
    fn statements_made_in_a_batch_share_its_timestamp() {
        let (counting, signer) = counting();
        let stamps = batch(&signer, |s| {
            let first = crate::timestamp();
            std::thread::sleep(std::time::Duration::from_millis(1100));
            s.sign(&first.to_le_bytes())?;
            Ok((first, crate::timestamp()))
        })
        .unwrap();
        assert_eq!(stamps.0, stamps.1);
        assert_eq!(counting.singles.load(Ordering::SeqCst), 0, "no single sign");
        assert!(
            crate::timestamp() >= stamps.0,
            "the clock is back afterwards"
        );
    }

    #[test]
    fn a_software_key_builds_once_with_its_own_timestamps() {
        let key = SigningKey::generate().unwrap();
        let signer = software(key.clone());
        let calls = AtomicUsize::new(0);
        let signature = batch(&signer, |s| {
            calls.fetch_add(1, Ordering::SeqCst);
            s.sign(b"entry")
        })
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "one pass");
        assert_eq!(signature, key.sign(b"entry").unwrap());
        // No fixed time is in force for it.
        let fixed = batch(&signer, |_| Ok(crate::BATCH_TIMESTAMP.with(|c| c.get()))).unwrap();
        assert_eq!(fixed, None);
    }

    /// A signer whose key service is down.
    struct Down;

    impl MessageSigner for Down {
        fn public_key(&self) -> Result<VerificationKey> {
            Err(SignerUnavailable::new("the key service is unreachable").into())
        }
        fn sign(&self, _bytes: &[u8]) -> Result<Signature> {
            Err(SignerUnavailable::new("the key service is unreachable").into())
        }
        fn batches(&self) -> bool {
            true
        }
    }

    #[test]
    fn an_unavailable_signer_is_seen_through_context() {
        use anyhow::Context;
        let unavailable = SignerUnavailable::because(
            "the key service is unreachable",
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused"),
        );
        let error = Err::<(), _>(unavailable)
            .context("Error building keycloak event message")
            .context("Error building the record")
            .unwrap_err();
        assert!(is_signer_unavailable(&error));
        assert_eq!(
            format!("{error:#}"),
            "Error building the record: Error building keycloak event message: \
             the key service is unreachable: refused"
        );
        assert!(!is_signer_unavailable(&anyhow!("the key service is unreachable")));
        // Given as the context of another error.
        let as_context = anyhow!("error sending request")
            .context(SignerUnavailable::new("the key service is unreachable"))
            .context("Error building keycloak event message");
        assert!(is_signer_unavailable(&as_context));
        assert!(!is_signer_unavailable(
            &anyhow!("refused").context("Error building keycloak event message")
        ));
    }

    #[test]
    fn an_unavailable_signer_stays_unavailable_through_a_batch_and_a_message() {
        let signer: SharedSigner = Arc::new(Down);
        let error = batch(&signer, |s| s.sign(b"entry")).unwrap_err();
        assert!(is_signer_unavailable(&error));
        let error = crate::signature::sign(
            signer.as_ref(),
            crate::signature::Domain::Statement,
            b"statement",
        )
        .unwrap_err();
        assert!(is_signer_unavailable(&error));
    }

    #[test]
    fn an_empty_batch_asks_nothing_and_errors_propagate() {
        let (counting, signer) = counting();
        let none: Vec<Signature> = batch(&signer, |_| Ok(Vec::new())).unwrap();
        assert!(none.is_empty());
        assert_eq!(counting.batches.load(Ordering::SeqCst), 0);
        assert!(batch(&signer, |_| -> Result<()> {
            Err(anyhow!("builder failed"))
        })
        .is_err());
    }
}
