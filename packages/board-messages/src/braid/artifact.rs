// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashSet;
use std::iter::FromIterator;
use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use strand::shuffler_product::StrandRectangle;
use strand::zkp::{ChaumPedersen, Schnorr};

use crate::braid::newtypes::PROTOCOL_MANAGER_INDEX;
use crate::braid::newtypes::{BatchNumber, MixNumber};

use strand::serialization::StrandSerialize;
use strand::shuffler::ShuffleProof;
use strand::signature::StrandSignaturePk;
use strand::symm;
use strand::{context::Ctx, elgamal::Ciphertext};

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Configuration<C: Ctx> {
    pub id: u128,
    pub protocol_manager: StrandSignaturePk,
    pub trustees: Vec<StrandSignaturePk>,
    pub threshold: usize,
    pub phantom: PhantomData<C>,
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
        };
        assert!(c.is_valid());

        c
    }

    pub fn is_valid(&self) -> bool {
        let unique: HashSet<StrandSignaturePk> = HashSet::from_iter(self.trustees.clone());

        (unique.len() == self.trustees.len())
            && (self.trustees.len() > 1
                && self.trustees.len() <= crate::braid::newtypes::MAX_TRUSTEES)
            && (self.threshold > 1 && self.threshold <= self.trustees.len())
    }

    pub fn get_trustee_position(&self, trustee_pk: &StrandSignaturePk) -> Option<usize> {
        if trustee_pk == &self.protocol_manager {
            Some(PROTOCOL_MANAGER_INDEX)
        } else {
            self.trustees.iter().position(|t| t == trustee_pk)
        }
    }

    pub fn label(&self, batch: BatchNumber, suffix: String) -> Vec<u8> {
        let mut ret = vec![];
        ret.extend(self.id.to_le_bytes());
        ret.extend(batch.to_le_bytes());
        ret.extend(suffix.len().to_le_bytes());
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
/*
// For some reason, deriving these does not work
impl<C: Ctx> BorshSerialize for Channel<C> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let mut bytes: Vec<Vec<u8>> = vec![];
        bytes.push(borsh::to_vec(&self.channel_pk)?);
        bytes.push(borsh::to_vec(&self.pk_proof)?);
        bytes.push(borsh::to_vec(&self.encrypted_channel_sk)?);

        bytes.serialize(writer)
    }
}

impl<C: Ctx> BorshDeserialize for Channel<C> {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let bytes = <Vec<Vec<u8>>>::deserialize_reader(reader)?;
        let channel_pk = C::E::try_from_slice(&bytes[0])?;
        let pk_proof = Schnorr::<C>::try_from_slice(&bytes[1])?;
        let encrypted_channel_sk = symm::EncryptionData::try_from_slice(&bytes[2])?;

        Ok(Channel {
            channel_pk,
            pk_proof,
            encrypted_channel_sk
        })
    }
}*/

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Shares<C: Ctx> {
    // Commitments to the coefficients of the generated polynomial
    pub commitments: Vec<C::E>,
    // One vector of bytes per trustee, including the share sent to
    // itself. The bytes are the serialization of the ElGamal
    // encryption of the share. See Ctx::encrypt_exp.
    pub encrypted_shares: Vec<Vec<u8>>,
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

/*
// For some reason, deriving these does not work
impl<C: Ctx> BorshSerialize for Ballots<C> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let mut bytes: Vec<Vec<u8>> = vec![];
        bytes.push(borsh::to_vec(&self.ciphertexts)?);

        bytes.serialize(writer)
    }
}

impl<C: Ctx> BorshDeserialize for Ballots<C> {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let bytes = <Vec<Vec<u8>>>::deserialize_reader(reader)?;
        let ciphertexts = StrandVector::<Ciphertext<C>>::try_from_slice(&bytes[0])?;

        Ok(Ballots {
            ciphertexts
        })
    }
}*/

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

/*
// For some reason, deriving these does not work
impl<C: Ctx> BorshSerialize for Mix<C> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let mut bytes: Vec<Vec<u8>> = vec![];
        bytes.push(borsh::to_vec(&self.ciphertexts)?);
        bytes.push(borsh::to_vec(&self.proof)?);
        bytes.push(borsh::to_vec(&self.mix_number)?);

        bytes.serialize(writer)
    }
}

impl<C: Ctx> BorshDeserialize for Mix<C> {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let bytes = <Vec<Vec<u8>>>::deserialize_reader(reader)?;
        let ciphertexts = StrandVector::<Ciphertext<C>>::try_from_slice(&bytes[0])?;
        let proof = Option::<ShuffleProof::<C>>::try_from_slice(&bytes[1])?;
        let mix_number = MixNumber::try_from_slice(&bytes[2])?;

        Ok(Mix {
            ciphertexts,
            proof,
            mix_number
        })
    }
}
    */

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

/*
// For some reason, deriving these does not work
impl<C: Ctx> BorshSerialize for DecryptionFactors<C> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let mut bytes: Vec<Vec<u8>> = vec![];
        bytes.push(borsh::to_vec(&self.factors)?);
        bytes.push(borsh::to_vec(&self.proofs)?);

        bytes.serialize(writer)
    }
}

impl<C: Ctx> BorshDeserialize for DecryptionFactors<C> {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let bytes = <Vec<Vec<u8>>>::deserialize_reader(reader)?;
        let factors = StrandVector::<C::E>::try_from_slice(&bytes[0])?;
        let proofs = StrandVector::<ChaumPedersen<C>>::try_from_slice(&bytes[1])?;

        Ok(DecryptionFactors {
            factors,
            proofs,
        })
    }
}
    */

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
