// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashSet;
use std::iter::FromIterator;
use std::marker::PhantomData;

use super::newtypes::PROTOCOL_MANAGER_INDEX;

use cryptography::context::Context;
use cryptography::cryptosystem::elgamal::Ciphertext;
use cryptography::cryptosystem::naoryung;

use cryptography::dkgd::dealer::CheckingValue;
use cryptography::utils::serialization::{VDeserializable, VSerializable};
use cryptography::utils::signatures::SignatureScheme;
use cryptography::zkp::shuffle::ShuffleProof;
use cryptography::VSerializable as VSer;

#[derive(VSer)]
pub struct Configuration<C: Context> {
    pub id: u128,
    pub protocol_manager: <C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier,
    pub trustees: Vec<<C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier>,
    pub threshold: usize,
    pub ciphertext_width: usize,
    /// Per-trustee share-encryption public keys, one element per entry in
    /// `trustees` and in the same order. Peers encrypt DKG shares to these keys
    /// (§9.4).
    pub share_encryption_keys: Vec<C::Element>,
    pub phantom: PhantomData<C>,
}

impl<C: Context> Clone for Configuration<C> {
    fn clone(&self) -> Self {
        Configuration {
            id: self.id,
            protocol_manager: self.protocol_manager.clone(),
            trustees: self.trustees.clone(),
            threshold: self.threshold,
            ciphertext_width: self.ciphertext_width,
            share_encryption_keys: self.share_encryption_keys.clone(),
            phantom: PhantomData,
        }
    }
}

impl<C: Context> Configuration<C> {
    pub fn new(
        id: u128,
        protocol_manager: <C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier,
        trustees: Vec<<C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier>,
        threshold: usize,
        ciphertext_width: usize,
        share_encryption_keys: Vec<C::Element>,
        _phantom: PhantomData<C>,
    ) -> Configuration<C> {
        let c = Configuration {
            id,
            protocol_manager,
            trustees,
            threshold,
            ciphertext_width,
            share_encryption_keys,
            phantom: PhantomData,
        };
        assert!(c.is_valid());

        c
    }

    pub fn is_valid(&self) -> bool {
        let unique: HashSet<<C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier> =
            HashSet::from_iter(self.trustees.clone());

        (unique.len() == self.trustees.len())
            && (self.trustees.len() > 1 && self.trustees.len() <= super::newtypes::MAX_TRUSTEES)
            && (self.threshold > 1 && self.threshold <= self.trustees.len())
            && (self.ciphertext_width >= 1
                && self.ciphertext_width <= super::newtypes::MAX_CIPHERTEXT_WIDTH)
            && (self.share_encryption_keys.len() == self.trustees.len())
    }

    pub fn get_trustee_position(
        &self,
        trustee_pk: &<C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier,
    ) -> Option<usize> {
        if trustee_pk == &self.protocol_manager {
            Some(PROTOCOL_MANAGER_INDEX as usize)
        } else {
            self.trustees.iter().position(|t| t == trustee_pk)
        }
    }
}

#[derive(Debug, VSer)]
pub struct Shares<C: Context> {
    // Commitments to the coefficients of the generated polynomial, aka the
    // checking values, each carrying a Schnorr proof of knowledge of its
    // exponent (§7). Recipients verify every proof; any failure halts.
    pub commitments: Vec<CheckingValue<C>>,
    // One vector of bytes per trustee, including the share sent to
    // itself. The bytes are the serialization of the ElGamal
    // encryption of the share. See  CryptographicGroup::encrypt_scalar.
    pub encrypted_shares: Vec<Vec<u8>>,
}

#[derive(Debug, VSer)]
pub struct DkgPublicKey<C: Context> {
    pub pk: C::Element,
    pub verification_keys: Vec<C::Element>,
}

impl<C: Context> DkgPublicKey<C> {
    pub fn new(pk: C::Element, verification_keys: Vec<C::Element>) -> DkgPublicKey<C> {
        DkgPublicKey {
            pk,
            verification_keys,
        }
    }
}

/// The tally input: Naor-Yung ballot ciphertexts (PROTOCOL.md §3.6, §5.5).
/// Every trustee independently verifies each ciphertext's well-formedness
/// proof and strips it to its ElGamal part at first-mix engagement; the
/// stripped list is derived locally and never posted.
#[derive(Debug, VSer)]
pub struct Ballots<C: Context, const W: usize> {
    pub ciphertexts: Vec<naoryung::Ciphertext<C, W>>,
}

impl<C: Context, const W: usize> Ballots<C, W> {
    pub fn new(ciphertexts: Vec<naoryung::Ciphertext<C, W>>) -> Ballots<C, W> {
        Ballots { ciphertexts }
    }
}

#[derive(Clone)]
pub struct Mix<C: Context, const W: usize> {
    pub ciphertexts: Vec<Ciphertext<C, W>>,
    pub proof: Option<ShuffleProof<C, W>>,
}

impl<C: Context, const W: usize> VSerializable for Mix<C, W> {
    fn ser(&self) -> Vec<u8> {
        (&self.ciphertexts, &self.proof).ser()
    }
}

impl<C: Context, const W: usize> VDeserializable for Mix<C, W> {
    fn deser(buffer: &[u8]) -> Result<Self, cryptography::utils::error::Error> {
        let (ciphertexts, proof) =
            <(Vec<Ciphertext<C, W>>, Option<ShuffleProof<C, W>>)>::deser(buffer)?;
        Ok(Mix { ciphertexts, proof })
    }
}

impl<C: Context, const W: usize> Mix<C, W> {
    pub fn new(ciphertexts: Vec<Ciphertext<C, W>>, proof: ShuffleProof<C, W>) -> Mix<C, W> {
        Mix {
            ciphertexts,
            proof: Some(proof),
        }
    }
    /// A null mix (empty input ⇒ empty output, no proof).
    pub fn null() -> Mix<C, W> {
        Mix {
            ciphertexts: vec![],
            proof: None,
        }
    }
}

/// Partial decryption data for transmission over the wire: one decryption
/// factor per ciphertext, and a single proof covering all of them.
///
/// Re-exported from the cryptography layer rather than redefined here. It used
/// to be a distinct message-layer type because the crypto layer's equivalent
/// bundled the participant's position, which must **not** travel in the body —
/// it is determined by the message signature. That is no longer so: the position
/// is attached by
/// [`AttributedDecryption`][`cryptography::dkgd::recipient::AttributedDecryption`]
/// at the point of use, so the published form is identical on both sides of the
/// boundary and one type serves.
pub use cryptography::dkgd::recipient::PartialDecryption;

#[derive(Debug, VSer)]
pub struct Plaintexts<C: Context, const W: usize>(pub Vec<[C::Element; W]>);

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

impl<C: Context> std::fmt::Debug for Configuration<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes = self.ser();
        let hashed = super::newtypes::hash_bytes(&bytes);
        write!(
            f,
            "hash={:?}, trustees={:?}, pm={:?}, threshold={}",
            hex::encode(hashed)[0..10].to_string(),
            self.trustees,
            self.protocol_manager,
            self.threshold
        )
    }
}

impl<C: Context, const W: usize> std::fmt::Debug for Mix<C, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Mix {{ ciphertexts: {}, proof: {} }}",
            self.ciphertexts.len(),
            self.proof.is_some()
        )
    }
}
