// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashSet;
use std::iter::FromIterator;
use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use strand::shuffler_product::StrandRectangle;
use strand::zkp::{ChaumPedersen, Schnorr};

use crate::messages::newtypes::PROTOCOL_MANAGER_INDEX;
use crate::messages::newtypes::{BatchNumber, MixNumber};

use strand::serialization::StrandSerialize;
use strand::shuffler::ShuffleProof;
use strand::signature::StrandSignaturePk;
use strand::symm;
use strand::{context::Ctx, elgamal::Ciphertext};

/// Whether each dealer must prove knowledge of the discrete log of its
/// first share commitment during distributed key generation.
///
/// Configurations encoded without a policy decode as `Legacy`, which keeps
/// existing boards readable. `Configuration::new` uses `SchnorrPok`.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[borsh(use_discriminant = true)]
pub enum DkgCommitmentProofPolicy {
    Legacy = 0,
    SchnorrPok = 1,
}

#[derive(Clone)]
pub struct Configuration<C: Ctx> {
    pub id: u128,
    pub protocol_manager: StrandSignaturePk,
    pub trustees: Vec<StrandSignaturePk>,
    pub threshold: usize,
    pub phantom: PhantomData<C>,
    pub dkg_commitment_proof_policy: DkgCommitmentProofPolicy,
}

/// The policy is appended only when it is not `Legacy`, so legacy
/// configurations keep their exact encoding and hash.
impl<C: Ctx> BorshSerialize for Configuration<C> {
    /// Writes a `Configuration`, with the optional trailing field only when it is present.
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.id.serialize(writer)?;
        self.protocol_manager.serialize(writer)?;
        self.trustees.serialize(writer)?;
        self.threshold.serialize(writer)?;
        match self.dkg_commitment_proof_policy {
            DkgCommitmentProofPolicy::Legacy => Ok(()),
            policy => policy.serialize(writer),
        }
    }
}

/// Any byte left after the threshold is read as the policy, so a
/// Configuration must always be encoded on its own.
impl<C: Ctx> BorshDeserialize for Configuration<C> {
    /// Reads a `Configuration`, with the optional trailing field only when it is present.
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let id = u128::deserialize_reader(reader)?;
        let protocol_manager = StrandSignaturePk::deserialize_reader(reader)?;
        let trustees = Vec::<StrandSignaturePk>::deserialize_reader(reader)?;
        let threshold = usize::deserialize_reader(reader)?;

        let mut policy_byte = [0u8; 1];
        let dkg_commitment_proof_policy = if reader.read(&mut policy_byte)? == 0 {
            DkgCommitmentProofPolicy::Legacy
        } else {
            match DkgCommitmentProofPolicy::try_from_slice(&policy_byte)? {
                DkgCommitmentProofPolicy::Legacy => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Legacy dkg commitment proof policy must not be encoded",
                    ));
                }
                policy => policy,
            }
        };

        Ok(Configuration {
            id,
            protocol_manager,
            trustees,
            threshold,
            phantom: PhantomData,
            dkg_commitment_proof_policy,
        })
    }
}

impl<C: Ctx> Configuration<C> {
    pub fn new(
        id: u128,
        protocol_manager: StrandSignaturePk,
        trustees: Vec<StrandSignaturePk>,
        threshold: usize,
        _phantom: PhantomData<C>,
    ) -> Configuration<C> {
        let c = Configuration {
            id,
            protocol_manager,
            trustees,
            threshold,
            phantom: PhantomData,
            dkg_commitment_proof_policy: DkgCommitmentProofPolicy::SchnorrPok,
        };
        assert!(c.is_valid());

        c
    }

    pub fn is_valid(&self) -> bool {
        let unique: HashSet<StrandSignaturePk> = HashSet::from_iter(self.trustees.clone());

        (unique.len() == self.trustees.len())
            && (self.trustees.len() > 1
                && self.trustees.len() <= crate::messages::newtypes::MAX_TRUSTEES)
            && (self.threshold > 1 && self.threshold <= self.trustees.len())
    }

    pub fn get_trustee_position(&self, trustee_pk: &StrandSignaturePk) -> Option<usize> {
        if trustee_pk == &self.protocol_manager {
            Some(PROTOCOL_MANAGER_INDEX as usize)
        } else {
            self.trustees.iter().position(|t| t == trustee_pk)
        }
    }

    pub fn label(&self, batch: BatchNumber, suffix: String) -> Vec<u8> {
        let mut ret = vec![];
        ret.extend(self.id.to_le_bytes());
        ret.extend(batch.to_le_bytes());
        // platform-independent value (cannot use usize as it may differ)
        let suffix_len = suffix.len() as u64;
        ret.extend(suffix_len.to_le_bytes());
        ret.extend(suffix.as_bytes());

        ret
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Channel<C: Ctx> {
    // The public key (as an element) with which other trustees will encrypt shares sent to the originator of this ShareTransport
    pub channel_pk: C::E,
    pub pk_proof: Schnorr<C>,
    pub encrypted_channel_sk: symm::EncryptionData,
}
impl<C: Ctx> Channel<C> {
    pub fn new(
        channel_pk: C::E,
        pk_proof: Schnorr<C>,
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
#[derive(BorshSerialize, BorshDeserialize)]
pub struct TrusteeShareData<C: Ctx> {
    pub channel: Channel<C>,
    pub shares: Vec<Shares<C>>,
}

#[derive(Debug)]
pub struct Shares<C: Ctx> {
    // Commitments to the coefficients of the generated polynomial
    pub commitments: Vec<C::E>,
    // One vector of bytes per trustee, including the share sent to
    // itself. The bytes are the serialization of the ElGamal
    // encryption of the share. See Ctx::encrypt_exp.
    pub encrypted_shares: Vec<Vec<u8>>,
    // Proof of knowledge of the discrete log of the first commitment,
    // see DkgCommitmentProofPolicy
    pub commitment_proof: Option<Schnorr<C>>,
}

/// Leading value of Shares that carry a commitment proof. Shares without
/// one keep the legacy encoding, which starts with the number of
/// commitments and so never takes this value.
const SHARES_WITH_COMMITMENT_PROOF: u32 = u32::MAX;

impl<C: Ctx> BorshSerialize for Shares<C> {
    /// Writes a `Shares`, with the optional trailing field only when it is present.
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match &self.commitment_proof {
            Some(proof) => {
                SHARES_WITH_COMMITMENT_PROOF.serialize(writer)?;
                self.commitments.serialize(writer)?;
                self.encrypted_shares.serialize(writer)?;
                proof.serialize(writer)
            }
            None => {
                self.commitments.serialize(writer)?;
                self.encrypted_shares.serialize(writer)
            }
        }
    }
}

impl<C: Ctx> BorshDeserialize for Shares<C> {
    /// Reads a `Shares`, with the optional trailing field only when it is present.
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let prefix = u32::deserialize_reader(reader)?;
        if prefix == SHARES_WITH_COMMITMENT_PROOF {
            Ok(Shares {
                commitments: Vec::<C::E>::deserialize_reader(reader)?,
                encrypted_shares: Vec::<Vec<u8>>::deserialize_reader(reader)?,
                commitment_proof: Some(Schnorr::<C>::deserialize_reader(reader)?),
            })
        } else {
            let commitments = (0..prefix)
                .map(|_| C::E::deserialize_reader(reader))
                .collect::<std::io::Result<Vec<C::E>>>()?;
            Ok(Shares {
                commitments,
                encrypted_shares: Vec::<Vec<u8>>::deserialize_reader(reader)?,
                commitment_proof: None,
            })
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct DkgPublicKey<C: Ctx> {
    pub pk: C::E,
    pub verification_keys: Vec<C::E>,
}
impl<C: Ctx> DkgPublicKey<C> {
    pub fn new(pk: C::E, verification_keys: Vec<C::E>) -> DkgPublicKey<C> {
        DkgPublicKey {
            pk,
            verification_keys,
        }
    }
}

use strand::serialization::StrandVector;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Ballots<C: Ctx> {
    pub ciphertexts: StrandVector<Ciphertext<C>>,
}
impl<C: Ctx> Ballots<C> {
    pub fn new(ciphertexts: Vec<Ciphertext<C>>) -> Ballots<C> {
        Ballots {
            ciphertexts: StrandVector(ciphertexts),
        }
    }
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct Mix<C: Ctx> {
    pub ciphertexts: StrandVector<Ciphertext<C>>,
    pub proof: Option<ShuffleProof<C>>,
    pub mix_number: MixNumber,
}
impl<C: Ctx> Mix<C> {
    pub fn new(
        ciphertexts: Vec<Ciphertext<C>>,
        proof: ShuffleProof<C>,
        mix_number: MixNumber,
    ) -> Mix<C> {
        Mix {
            ciphertexts: StrandVector(ciphertexts),
            proof: Some(proof),
            mix_number,
        }
    }
    pub fn null(mix_number: MixNumber) -> Mix<C> {
        Mix {
            ciphertexts: StrandVector(vec![]),
            proof: None,
            mix_number,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct DecryptionFactors<C: Ctx> {
    pub factors: StrandVector<C::E>,
    pub proofs: StrandVector<ChaumPedersen<C>>,
}
impl<C: Ctx> DecryptionFactors<C> {
    pub fn new(factors: Vec<C::E>, proofs: StrandVector<ChaumPedersen<C>>) -> DecryptionFactors<C> {
        DecryptionFactors {
            factors: StrandVector(factors),
            proofs,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Plaintexts<C: Ctx>(pub StrandVector<C::P>);

///////////////////////////////////////////////////////////////////////////
// Wide artifacts
///////////////////////////////////////////////////////////////////////////

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct BallotsWide<C: Ctx> {
    pub ciphertexts: StrandRectangle<Ciphertext<C>>,
}
impl<C: Ctx> BallotsWide<C> {
    pub fn new(ciphertexts: StrandRectangle<Ciphertext<C>>) -> BallotsWide<C> {
        BallotsWide { ciphertexts }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct MixWide<C: Ctx> {
    pub ciphertexts: StrandRectangle<Ciphertext<C>>,
    pub proof: Option<ShuffleProof<C>>,
    pub mix_number: MixNumber,
}
impl<C: Ctx> MixWide<C> {
    pub fn new(
        ciphertexts: StrandRectangle<Ciphertext<C>>,
        proof: ShuffleProof<C>,
        mix_number: MixNumber,
    ) -> MixWide<C> {
        MixWide {
            ciphertexts,
            proof: Some(proof),
            mix_number,
        }
    }
    pub fn null(mix_number: MixNumber) -> MixWide<C> {
        let c = StrandRectangle::new(vec![vec![]]).expect("impossible");

        MixWide {
            ciphertexts: c,
            proof: None,
            mix_number,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct DecryptionFactorsWide<C: Ctx> {
    pub factors: StrandRectangle<C::E>,
    pub proofs: StrandRectangle<ChaumPedersen<C>>,
}
impl<C: Ctx> DecryptionFactorsWide<C> {
    pub fn new(
        factors: StrandRectangle<C::E>,
        proofs: StrandRectangle<ChaumPedersen<C>>,
    ) -> DecryptionFactorsWide<C> {
        DecryptionFactorsWide { factors, proofs }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct PlaintextsWide<C: Ctx>(pub StrandRectangle<C::P>);

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

impl<C: Ctx> std::fmt::Debug for Configuration<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hashed = strand::hash::hash(&self.strand_serialize().unwrap()).unwrap();
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

impl<C: Ctx> std::fmt::Debug for Channel<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "channel_pk={:?},", self.channel_pk,)
    }
}

impl<C: Ctx> std::fmt::Debug for Mix<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mix_number={:?}", self.mix_number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strand::backend::ristretto::RistrettoCtx;
    use strand::serialization::StrandDeserialize;
    use strand::signature::StrandSignatureSk;
    use strand::zkp::Zkp;

    type C = RistrettoCtx;

    #[derive(BorshSerialize)]
    struct LegacyConfiguration {
        id: u128,
        protocol_manager: StrandSignaturePk,
        trustees: Vec<StrandSignaturePk>,
        threshold: usize,
    }

    /// Returns a fixed signature public key for the encoding tests.
    fn signature_pk() -> StrandSignaturePk {
        StrandSignaturePk::from_sk(&StrandSignatureSk::generate().unwrap()).unwrap()
    }

    /// Encodes a configuration the way it was encoded before the commitment proof policy existed.
    fn legacy_configuration_bytes() -> Vec<u8> {
        LegacyConfiguration {
            id: 7,
            protocol_manager: signature_pk(),
            trustees: vec![signature_pk(), signature_pk(), signature_pk()],
            threshold: 2,
        }
        .strand_serialize()
        .unwrap()
    }

    /// Builds a `Shares` message, with or without a commitment proof.
    fn shares(commitment_proof: bool) -> Shares<C> {
        let ctx = C::default();
        let (coefficients, commitments) = strand::threshold::gen_coefficients(2, &ctx);
        let commitment_proof = commitment_proof.then(|| {
            Zkp::new(&ctx)
                .schnorr_prove(&coefficients[0], &commitments[0], None, b"label")
                .unwrap()
        });

        Shares {
            commitments,
            encrypted_shares: vec![vec![1, 2, 3], vec![4, 5]],
            commitment_proof,
        }
    }

    /// Asserts that two `Shares` messages carry identical fields.
    fn assert_same_shares(left: &Shares<C>, right: &Shares<C>) {
        assert_eq!(left.commitments, right.commitments);
        assert_eq!(left.encrypted_shares, right.encrypted_shares);
        assert_eq!(left.commitment_proof, right.commitment_proof);
    }

    /// A configuration encoded without the policy byte decodes as `Legacy` and re-encodes to the same bytes.
    #[test]
    fn configuration_without_policy_decodes_as_legacy_with_same_encoding() {
        let bytes = legacy_configuration_bytes();

        let cfg = Configuration::<C>::strand_deserialize(&bytes).unwrap();

        assert_eq!(
            cfg.dkg_commitment_proof_policy,
            DkgCommitmentProofPolicy::Legacy
        );
        assert_eq!(cfg.threshold, 2);
        assert_eq!(cfg.strand_serialize().unwrap(), bytes);
    }

    /// A configuration created with `Configuration::new` keeps the `SchnorrPok` policy through encoding and decoding.
    #[test]
    fn new_configuration_round_trips_schnorr_pok_policy() {
        let cfg = Configuration::<C>::new(
            7,
            signature_pk(),
            vec![signature_pk(), signature_pk()],
            2,
            PhantomData,
        );
        let bytes = cfg.strand_serialize().unwrap();

        let decoded = Configuration::<C>::strand_deserialize(&bytes).unwrap();

        assert_eq!(
            cfg.dkg_commitment_proof_policy,
            DkgCommitmentProofPolicy::SchnorrPok
        );
        assert_eq!(
            decoded.dkg_commitment_proof_policy,
            DkgCommitmentProofPolicy::SchnorrPok
        );
        assert_eq!(decoded.strand_serialize().unwrap(), bytes);
    }

    /// An explicitly encoded `Legacy` policy byte is rejected, so each configuration has one encoding.
    #[test]
    fn configuration_rejects_encoded_legacy_policy() {
        let mut bytes = legacy_configuration_bytes();
        bytes.push(DkgCommitmentProofPolicy::Legacy as u8);

        assert!(Configuration::<C>::strand_deserialize(&bytes).is_err());
    }

    /// An unknown policy byte is rejected.
    #[test]
    fn configuration_rejects_unknown_policy() {
        let mut bytes = legacy_configuration_bytes();
        bytes.push(u8::MAX);

        assert!(Configuration::<C>::strand_deserialize(&bytes).is_err());
    }

    /// `Shares` without a proof keep the encoding they had before the proof was added.
    #[test]
    fn shares_without_proof_keep_legacy_encoding() {
        let shares = shares(false);
        let legacy_bytes = (shares.commitments.clone(), shares.encrypted_shares.clone())
            .strand_serialize()
            .unwrap();

        let decoded = Shares::<C>::strand_deserialize(&legacy_bytes).unwrap();

        assert_same_shares(&decoded, &shares);
        assert_eq!(shares.strand_serialize().unwrap(), legacy_bytes);
    }

    /// `Shares` with and without a proof decode correctly when stored back to back.
    #[test]
    fn shares_with_and_without_proof_round_trip_in_sequence() {
        let sequence = vec![shares(true), shares(false), shares(true)];
        let bytes = sequence.strand_serialize().unwrap();

        let decoded = Vec::<Shares<C>>::strand_deserialize(&bytes).unwrap();

        assert_eq!(decoded.len(), sequence.len());
        for (left, right) in decoded.iter().zip(sequence.iter()) {
            assert_same_shares(left, right);
        }
    }
}
