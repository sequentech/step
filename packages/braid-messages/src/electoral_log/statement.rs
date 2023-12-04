use borsh::{BorshSerialize, BorshDeserialize};
use strum::Display;

use crate::electoral_log::newtypes::ContextHash;
use crate::electoral_log::newtypes::Timestamp;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Statement {
    pub head: StatementHead,
    pub body: StatementBody,
}
impl Statement {
    pub fn new(head: StatementHead, body: StatementBody) -> Statement {
        Statement {
            head,
            body,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct StatementHead {
    pub context: ContextHash,
    pub kind: StatementType,
    pub timestamp: Timestamp
}
impl StatementHead {
    pub fn from_body(context: ContextHash, body: &StatementBody) -> Self {
        let kind = match body {
            StatementBody::One => StatementType::One,
        };
        let timestamp = instant::now() as u64;

        StatementHead {
            context,
            kind,
            timestamp,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum StatementBody {
    One
}

#[derive(BorshSerialize, BorshDeserialize, Display)]
pub enum StatementType {
    One
}