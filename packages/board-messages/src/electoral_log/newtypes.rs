use borsh::{BorshDeserialize, BorshSerialize};
use strand::hash::Hash;

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ContextHash(pub Hash);
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PseudonymHash(pub Hash);

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CastVoteHash(pub Hash);

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ContestHash(pub Hash);

pub type Timestamp = u64;
