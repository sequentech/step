use borsh::{BorshDeserialize, BorshSerialize};
use strand::hash::Hash;

#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct EventIdString(pub String);
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ElectionIdString(pub Option<String>);
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ContestIdString(pub String);
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct BallotPublicationIdString(pub String);
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct CastVoteErrorString(pub String);

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PseudonymHash(pub Hash);

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct CastVoteHash(pub Hash);

pub type Timestamp = u64;
