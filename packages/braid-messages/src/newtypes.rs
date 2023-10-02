use borsh::{BorshDeserialize, BorshSerialize};
use anyhow::Result;

use strand::context::Ctx;
use strand::serialization::StrandSerialize;
use crate::artifact::Configuration;
use strand::hash::Hash;

pub const MAX_TRUSTEES: usize = 12;
pub const PROTOCOL_MANAGER_INDEX: usize = 1000;
pub const VERIFIER_INDEX: usize = 2000;
pub const NULL_TRUSTEE: usize = 1001;


#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConfigurationHash(pub Hash);
impl ConfigurationHash {
    pub fn from_configuration<C: Ctx>(
        configuration: &Configuration<C>,
    ) -> Result<ConfigurationHash> {
        let bytes = configuration.strand_serialize()?;
        let hash = strand::hash::hash(&bytes)?;
        Ok(ConfigurationHash(strand::util::to_hash_array(&hash)?))
    }
}
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ChannelHash(pub Hash);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ChannelsHashes(pub THashes);
/*
use crate::util::dbg_hashes;
impl std::fmt::Debug for ChannelsHashes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hashes={:?}", dbg_hashes(&self.0))
    }
}
*/


#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SharesHash(pub Hash);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SharesHashes(pub THashes);
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PublicKeyHash(pub Hash);
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
// The ciphertexts hash is used to refer to ballots and mix artifacts.
// This allows accessing either one when pointing to a source of
// ciphertexts (ballots or mix). The same typed hash is propagated
// all the way from Ballots to DecryptionFactors predicates.
pub struct CiphertextsHash(pub Hash);

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct DecryptionFactorsHash(pub Hash);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct DecryptionFactorsHashes(pub THashes);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct MixingHashes(pub THashes);
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PlaintextsHash(pub Hash);

// 0-based
pub type TrusteePosition = usize;
// 1-based
pub type Threshold = usize;
pub type TrusteeCount = usize;
// 1-based _elements_
pub type TrusteeSet = [usize; MAX_TRUSTEES];

// 1-based, the position in the mixing chain (note this is not the same as the
// position of the mixing trustee, since active trustees are set by the ballots artifact)
pub type MixNumber = usize;

pub type BatchNumber = usize;
pub type Timestamp = u64;

pub type THashes = [Hash; MAX_TRUSTEES];
