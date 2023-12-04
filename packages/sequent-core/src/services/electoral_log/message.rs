use anyhow::{anyhow, Result};
use borsh::{BorshSerialize, BorshDeserialize};

// use immu_board::BoardMessage;
use strand::signature::StrandSignature;
use strand::signature::StrandSignaturePk;
use strand::serialization::StrandSerialize;
use crate::services::electoral_log::statement::Statement;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Message {
    pub sender: Sender,
    pub signature: StrandSignature,
    pub statement: Statement,
    pub artifact: Option<Vec<u8>>,
}
/* 
impl TryFrom<Message> for BoardMessage {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<BoardMessage> {
        Ok(BoardMessage {
            id: 0,
            created: (instant::now() * 1000f64) as i64,
            statement_timestamp: (message.statement.get_timestamp() * 1000) as i64,
            statement_kind: message.statement.get_kind().to_string(),
            message: message.strand_serialize()?,
            signer_key: message.sender.pk.to_der_b64_string()?,
        })
    }
}*/

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Sender {
    pub name: String,
    pub pk: StrandSignaturePk,
}
impl Sender {
    pub fn new(name: String, pk: StrandSignaturePk) -> Sender {
        Sender { name, pk }
    }
}