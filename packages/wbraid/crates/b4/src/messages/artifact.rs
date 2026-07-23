// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashSet;
use std::iter::FromIterator;
use std::marker::PhantomData;

use crate::messages::newtypes::PROTOCOL_MANAGER_INDEX;
use crate::Hasher;
use cryptography::utils::hash::Hasher as HasherTrait;

use cryptography::context::Context;
use cryptography::cryptosystem::elgamal::Ciphertext;
use cryptography::dkgd::recipient::DecryptionFactor;
use cryptography::utils::serialization::{VDeserializable, VSerializable};
use cryptography::utils::signatures::SignatureScheme;
use cryptography::utils::symm;
use cryptography::zkp::schnorr::SchnorrProof;
use cryptography::zkp::shuffle::ShuffleProof;
use cryptography::VSerializable as VSer;
use sha3::Digest;

#[derive(VSer)]
pub struct Configuration<C: Context> {
    pub id: u128,
    pub protocol_manager: <C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier,
    pub trustees: Vec<<C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier>,
    pub threshold: usize,
    pub ciphertext_width: usize,
    /// Per-trustee share-encryption public keys (v0.6), one element per entry in
    /// `trustees` and in the same order. Peers encrypt DKG shares to these keys,
    /// replacing the former per-trustee `Channel` + symmetric key. Empty on the
    /// legacy (pre-v0.6) path, which still uses `Channel` messages.
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
        _phantom: PhantomData<C>,
    ) -> Configuration<C> {
        let c = Configuration {
            id,
            protocol_manager,
            trustees,
            threshold,
            ciphertext_width,
            share_encryption_keys: Vec::new(),
            phantom: PhantomData,
        };
        assert!(c.is_valid());

        c
    }

    /// Attaches the per-trustee share-encryption public keys (v0.6 DKG).
    ///
    /// `keys` must have one entry per trustee, in the same order as `trustees`.
    /// The public side lives on the board inside the `Configuration`; each
    /// trustee holds the matching secret scalar out of band (supplied at
    /// construction). Replaces the legacy `Channel` share-transport keys.
    pub fn with_share_encryption_keys(mut self, keys: Vec<C::Element>) -> Configuration<C> {
        assert_eq!(
            keys.len(),
            self.trustees.len(),
            "expected one share-encryption key per trustee"
        );
        self.share_encryption_keys = keys;
        self
    }

    pub fn is_valid(&self) -> bool {
        let unique: HashSet<<C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier> =
            HashSet::from_iter(self.trustees.clone());

        (unique.len() == self.trustees.len())
            && (self.trustees.len() > 1
                && self.trustees.len() <= crate::messages::newtypes::MAX_TRUSTEES)
            && (self.threshold > 1 && self.threshold <= self.trustees.len())
            && (self.ciphertext_width >= 1
                && self.ciphertext_width <= crate::messages::newtypes::MAX_CIPHERTEXT_WIDTH)
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

pub struct Channel<C: Context> {
    // The public key (as an element) with which other trustees will encrypt shares sent to the originator of this ShareTransport
    pub channel_pk: C::Element,
    pub pk_proof: SchnorrProof<C>,
    pub encrypted_channel_sk: symm::EncryptionData,
}

impl<C: Context> VSerializable for Channel<C> {
    fn ser(&self) -> Vec<u8> {
        (&self.channel_pk, &self.pk_proof, &self.encrypted_channel_sk).ser()
    }
}

impl<C: Context> VDeserializable for Channel<C> {
    fn deser(buffer: &[u8]) -> Result<Self, cryptography::utils::error::Error> {
        let (channel_pk, pk_proof, encrypted_channel_sk) =
            <(C::Element, SchnorrProof<C>, symm::EncryptionData)>::deser(buffer)?;
        Ok(Channel {
            channel_pk,
            pk_proof,
            encrypted_channel_sk,
        })
    }
}

impl<C: Context> Channel<C> {
    pub fn new(
        channel_pk: C::Element,
        pk_proof: SchnorrProof<C>,
        encrypted_channel_sk: symm::EncryptionData,
    ) -> Channel<C> {
        Channel {
            channel_pk,
            pk_proof,
            encrypted_channel_sk,
        }
    }
}

/// Share data downloaded by trustees into removable media.
///
/// The encrypted private key in the Channel serves to
/// decrypt the shares sent to the trustee.
///
/// Strictly speaking this is not an artifact posted to
/// bulletin board, but we define it here anyway.
#[derive(VSer)]
pub struct TrusteeShareData<C: Context> {
    pub channel: Channel<C>,
    pub shares: Vec<Shares<C>>,
}

#[derive(Debug, VSer)]
pub struct Shares<C: Context> {
    // Commitments to the coefficients of the generated polynomial
    pub commitments: Vec<C::Element>,
    // One vector of bytes per trustee, including the share sent to
    // itself. The bytes are the serialization of the ElGamal
    // encryption of the share. See Ctx::encrypt_exp.
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

#[derive(Debug, VSer)]
pub struct Ballots<C: Context, const W: usize> {
    pub ciphertexts: Vec<Ciphertext<C, W>>,
}

impl<C: Context, const W: usize> Ballots<C, W> {
    pub fn new(ciphertexts: Vec<Ciphertext<C, W>>) -> Ballots<C, W> {
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

/// Partial decryption data for transmission over the wire.
///
/// Contains decryption factors (value + proof pairs) without participant position.
/// The position is determined by the message signature, not the message content.
///
/// This is the message-layer representation. The cryptography layer uses
/// [`cryptography::dkgd::recipient::DecryptionFactors`] which includes the source position.
#[derive(Debug)]
pub struct PartialDecryption<C: Context, const W: usize> {
    pub factors: Vec<DecryptionFactor<C, W>>,
}

impl<C: Context, const W: usize> VSerializable for PartialDecryption<C, W> {
    fn ser(&self) -> Vec<u8> {
        self.factors.ser()
    }
}

impl<C: Context, const W: usize> VDeserializable for PartialDecryption<C, W> {
    fn deser(buffer: &[u8]) -> Result<Self, cryptography::utils::error::Error> {
        let factors = Vec::<DecryptionFactor<C, W>>::deser(buffer)?;
        Ok(PartialDecryption { factors })
    }
}

impl<C: Context, const W: usize> PartialDecryption<C, W> {
    pub fn new(factors: Vec<DecryptionFactor<C, W>>) -> PartialDecryption<C, W> {
        PartialDecryption { factors }
    }
}

#[derive(Debug, VSer)]
pub struct Plaintexts<C: Context, const W: usize>(pub Vec<[C::Element; W]>);

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

impl<C: Context> std::fmt::Debug for Configuration<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes = self.ser();
        let mut hasher = Hasher::hasher();
        hasher.update(&bytes);
        let hashed = hasher.finalize();
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

impl<C: Context> std::fmt::Debug for Channel<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "channel_pk={:?},", self.channel_pk,)
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
