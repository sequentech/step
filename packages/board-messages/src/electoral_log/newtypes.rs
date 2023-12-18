use borsh::{BorshDeserialize, BorshSerialize};
use strand::hash::Hash;

#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash)]
pub struct EventIdString(pub String);
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash)]
pub struct ElectionIdString(pub String);
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash)]
pub struct ContestIdString(pub String);
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash)]
pub struct BallotPublicationIdString(pub String);
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash)]
pub struct CastVoteErrorString(pub String);

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PseudonymHash(pub Hash);

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CastVoteHash(pub Hash);

pub type Timestamp = u64;
