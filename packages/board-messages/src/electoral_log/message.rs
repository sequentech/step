use anyhow::{anyhow, Result};
use borsh::{BorshSerialize, BorshDeserialize};

use immu_board::BoardMessage;
use strand::signature::StrandSignature;
use strand::signature::StrandSignaturePk;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignatureSk;

use crate::electoral_log::statement::Statement;
use crate::electoral_log::statement::StatementBody;
use crate::electoral_log::statement::StatementHead;

use crate::electoral_log::newtypes::ContextHash;

use super::newtypes::*;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Message {
    pub sender: Sender,
    pub sender_signature: StrandSignature,
    pub system_signature: StrandSignature,
    pub statement: Statement,
    pub artifact: Option<Vec<u8>>,
}
impl Message {
    pub fn cast_vote_message(context: ContextHash, contest_h: ContestHash, pseudonym_h: PseudonymHash, vote_h: CastVoteHash, sd: &SigningData) -> Result<Self> {
        let body = StatementBody::CastVote(contest_h, pseudonym_h, vote_h);
        let head = StatementHead::from_body(context, &body);
        let statement = Statement::new(head, body);

        Message::sign(statement, None, &sd.sender_sk, &sd.sender_name, &sd.system_sk)
    }
    
    pub fn sign(statement: Statement, artifact: Option<Vec<u8>>, sender_sk: &StrandSignatureSk, sender_name: &str, system_sk: &StrandSignatureSk) -> Result<Message> {
        let bytes = statement.strand_serialize()?;
        let sender_signature: StrandSignature = sender_sk.sign(&bytes)?;
        let system_signature: StrandSignature = system_sk.sign(&bytes)?;
        let sender_pk = StrandSignaturePk::from_sk(&sender_sk)?;
        let sender = Sender::new(sender_name.to_string(), sender_pk);
        
        Ok(Message {
            sender,
            sender_signature,
            system_signature,
            statement,
            artifact,
        })
    }

    pub fn verify(&self, system_pk: &StrandSignaturePk) -> Result<()> {
        let bytes = self.statement.strand_serialize()?;
        self.sender.pk.verify(&self.sender_signature, &bytes)?;
        system_pk.verify(&self.system_signature, &bytes)?;

        Ok(())

    }
}

impl TryFrom<Message> for BoardMessage {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<BoardMessage> {
        Ok(BoardMessage {
            id: 0,
            created: (instant::now() * 1000f64) as i64,
            statement_timestamp: (message.statement.head.timestamp * 1000) as i64,
            statement_kind: message.statement.head.kind.to_string(),
            message: message.strand_serialize()?,
            sender_pk: message.sender.pk.to_der_b64_string()?,
        })
    }
}

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

pub struct SigningData {
    sender_sk: StrandSignatureSk,
    sender_name: String,
    system_sk: StrandSignatureSk,
    
}
impl SigningData {
    pub fn new(sender_sk: StrandSignatureSk, sender_name: &str, system_sk: StrandSignatureSk) -> SigningData {
        SigningData { 
            sender_sk, sender_name: sender_name.to_string(), system_sk 
        }
    }
}