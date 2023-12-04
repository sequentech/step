use strand::hash::Hash;
use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ContextHash(pub Hash);
pub type Timestamp = u64;
