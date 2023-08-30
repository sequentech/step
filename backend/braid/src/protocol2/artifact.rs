use std::collections::HashSet;
use std::iter::FromIterator;
use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use chacha20poly1305::consts::U12;
use generic_array::GenericArray;

use crate::protocol2::datalog::NULL_TRUSTEE;
use crate::protocol2::datalog::{BatchNumber, MixNumber, TrusteePosition};
use crate::protocol2::predicate::TrusteeSet;
use crate::protocol2::PROTOCOL_MANAGER_INDEX;

use strand::serialization::StrandSerialize;
use strand::shuffler::ShuffleProof;
use strand::signature::StrandSignaturePk;
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
        assert!(trustees.len() > 1 && trustees.len() <= crate::protocol2::MAX_TRUSTEES);
        assert!(threshold > 1 && threshold <= trustees.len());

        let unique: HashSet<StrandSignaturePk> = HashSet::from_iter(trustees.clone());
        assert_eq!(unique.len(), trustees.len());

        let c = Configuration {
            id,
            protocol_manager,
            trustees,
            threshold,
            phantom: PhantomData,
        };
        c.validate()
    }

    pub fn validate(self) -> Self {
        assert!(self.trustees.len() > 1 && self.trustees.len() <= crate::protocol2::MAX_TRUSTEES);
        assert!(self.threshold > 1 && self.threshold <= self.trustees.len());

        self
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
pub(crate) struct Commitments<C: Ctx> {
    // Commitments to the coefficients of the generated polynomial
    pub(crate) commitments: Vec<C::E>,
    // Encryption of the serialization of the vector of coefficients of the polynomial (C::X's)
    pub(crate) encrypted_coefficients: EncryptedCoefficients,
    // Channel with which to send shares privately to the originator of these Commitments
    pub(crate) share_transport: ShareTransport<C>,
}
impl<C: Ctx> Commitments<C> {
    pub(crate) fn new(
        commitments: Vec<C::E>,
        encrypted_coefficients: EncryptedCoefficients,
        share_transport: ShareTransport<C>,
    ) -> Commitments<C> {
        Commitments {
            commitments,
            encrypted_coefficients,
            share_transport,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub(crate) struct EncryptedCoefficients {
    pub(crate) encrypted_coefficients: Vec<u8>,
    pub(crate) nonce: Vec<u8>,
}
impl EncryptedCoefficients {
    pub(crate) fn new(
        encrypted_coefficients: Vec<u8>,
        nonce: GenericArray<u8, U12>,
    ) -> EncryptedCoefficients {
        EncryptedCoefficients {
            encrypted_coefficients,
            nonce: nonce.to_vec(),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
// ElGamal key information used to send shares confidentially
pub(crate) struct ShareTransport<C: Ctx> {
    // The public key with which other trustees will encrypt shares sent to the originator of this ShareTransport
    pub(crate) pk: C::E,
    // The encrypted (symmetric) private key corresponding to the above
    pub(crate) encrypted_sk: Vec<u8>,
    // The nonce used for symmetric encryption
    pub(crate) nonce: Vec<u8>,
}
impl<C: Ctx> ShareTransport<C> {
    pub(crate) fn new(
        pk: C::E,
        encrypted_sk: Vec<u8>,
        nonce: GenericArray<u8, U12>,
    ) -> ShareTransport<C> {
        ShareTransport {
            pk,
            encrypted_sk,
            nonce: nonce.to_vec(),
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
// One vector of bytes per trustee, including the share sent to
// itself. The bytes are the serialization of the ElGamal
// encryption of the share. See Ctx::encrypt_exp.
pub(crate) struct Shares(pub(crate) Vec<Vec<u8>>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct DkgPublicKey<C: Ctx> {
    pub pk: C::E,
    pub verification_keys: Vec<C::E>,
}
impl<C: Ctx> DkgPublicKey<C> {
    pub(crate) fn new(pk: C::E, verification_keys: Vec<C::E>) -> DkgPublicKey<C> {
        DkgPublicKey {
            pk,
            verification_keys,
        }
    }
}

use strand::serialization::StrandVectorC;
use strand::serialization::StrandVectorCP;
use strand::serialization::StrandVectorE;
use strand::serialization::StrandVectorP;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Ballots<C: Ctx> {
    pub ciphertexts: StrandVectorC<C>,
    // The trustees to participate in the mix and decryption. There must be threshold # of them.
    // Each trustee is a number starting at 1 up to the the number of eligible
    // trustees as per the configuration. 0 is not a valid trustee. Remaining
    // slots of this fixed size array must be padded with Datalog::NULL_TRUSTEE
    pub selected_trustees: TrusteeSet,
}
impl<C: Ctx> Ballots<C> {
    pub fn new(
        ciphertexts: Vec<Ciphertext<C>>,
        selected_trustees: TrusteeSet,
        cfg: &Configuration<C>,
    ) -> Ballots<C> {
        let b = Ballots {
            ciphertexts: StrandVectorC(ciphertexts),
            selected_trustees,
        };
        b.validate(cfg)
    }

    pub fn validate(self, cfg: &Configuration<C>) -> Self {
        let mut selected = vec![];
        self.selected_trustees.iter().for_each(|s| {
            if *s != NULL_TRUSTEE {
                assert!(*s > 0 && *s <= cfg.trustees.len());
                selected.push(*s);
            }
        });

        let unique: HashSet<usize> = selected.into_iter().collect();
        assert!(unique.len() == cfg.threshold);

        self
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub(crate) struct Mix<C: Ctx> {
    pub ciphertexts: StrandVectorC<C>,
    pub proof: ShuffleProof<C>,
    pub mix_number: MixNumber,
    pub target_trustee: TrusteePosition,
}
impl<C: Ctx> Mix<C> {
    pub fn new(
        ciphertexts: Vec<Ciphertext<C>>,
        proof: ShuffleProof<C>,
        mix_number: MixNumber,
        target_trustee: TrusteePosition,
    ) -> Mix<C> {
        Mix {
            ciphertexts: StrandVectorC(ciphertexts),
            proof,
            mix_number,
            target_trustee,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct DecryptionFactors<C: Ctx> {
    pub factors: StrandVectorE<C>,
    pub proofs: StrandVectorCP<C>,
}
impl<C: Ctx> DecryptionFactors<C> {
    pub fn new(factors: Vec<C::E>, proofs: StrandVectorCP<C>) -> DecryptionFactors<C> {
        DecryptionFactors {
            factors: StrandVectorE(factors),
            proofs,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Plaintexts<C: Ctx>(pub StrandVectorP<C>);

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

impl<C: Ctx> std::fmt::Debug for Configuration<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hashed = strand::util::hash(&self.strand_serialize().unwrap());
        write!(
            f,
            "hash={:?}, #trustees={}, threshold={}",
            hex::encode(hashed)[0..10].to_string(),
            self.trustees.len(),
            self.threshold
        )
    }
}

impl<C: Ctx> std::fmt::Debug for Commitments<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "commitments={:?}, coefficients={:?}",
            self.commitments,
            hex::encode(&self.encrypted_coefficients.encrypted_coefficients)[0..10].to_string()
        )
    }
}

impl<C: Ctx> std::fmt::Debug for Mix<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mix_number={:?}, target_trustee={:?}",
            self.mix_number, self.target_trustee
        )
    }
}
