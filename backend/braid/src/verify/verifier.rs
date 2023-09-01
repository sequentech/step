use crate::protocol2::action::Message;
use crate::protocol2::artifact::Configuration;
use crate::protocol2::board::immudb::ImmudbBoard;
use crate::protocol2::datalog::{
    BatchNumber, ConfigurationHash, MixingHashes, PlaintextsHash, Predicate, DecryptionFactorsHashes, PublicKeyHash,
};
use crate::protocol2::predicate::CiphertextsHash;
use crate::protocol2::statement::{Batch, CiphertextsH, PlaintextsH, Statement, StatementType, DecryptionFactorsHs, ConfigurationH};
use crate::protocol2::trustee::Trustee;
use anyhow::{anyhow, Result};
use strand::context::Ctx;
use strand::serialization::StrandDeserialize;
use tracing::info;

pub struct VerifyingSession<C: Ctx> {
    trustee: Trustee<C>,
    board: ImmudbBoard,
}
impl<C: Ctx> VerifyingSession<C> {
    pub fn new(trustee: Trustee<C>, board: ImmudbBoard) -> VerifyingSession<C> {
        VerifyingSession { trustee, board }
    }

    pub async fn run(&mut self) -> Result<()> {
        // A verification result is function of 4 inputs
        // (public key, ballots, plaintexts, trustees)

        // The following must be established
        // 1) The PK was correctly generated from the information published by the TRUSTEES
        // 2) The PK is associated to the BALLOTS according to the protocol managers signature
        // 3) The BALLOTS link to some output ciphertexts with verified mixes
        // 4) The output ciphertexts are correctly decrypted to the PLAINTEXTS with respect to the PK (the verification keys) 

        
        
        info!("Verifying..");

        let messages = self.board.get_messages(-1).await?;
        let cfg_message: Vec<Message> = messages
            .clone()
            .into_iter()
            .filter(|m| m.statement.get_kind() == StatementType::Configuration)
            .collect();

        assert_eq!(cfg_message.len(), 1);

        let cfg_bytes = cfg_message
            .first()
            .as_ref()
            .unwrap()
            .artifact
            .as_ref()
            .unwrap();
        let cfg = Configuration::<C>::strand_deserialize(&cfg_bytes)?;

        let (messages, mut predicates) = self.trustee.verify(messages)?;

        for message in messages.clone() {
            let predicate = Predicate::from_statement::<C>(
                &message.statement,
                crate::protocol2::VERIFIER_INDEX,
                &cfg,
            );
            predicates.push(predicate);
        }

        let (targets, verified) = crate::verify::v::S.run(&predicates);

        info!("Targets: {:?}", targets);
        info!("Verified: {:?}", verified);

        Ok(())
    }
}

use crate::util::dbg_hash;
impl std::fmt::Debug for crate::verify::v::Target  {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Verification Target
            Configuration = {}, 
            Batch = {}, 
            Public Key = {}, 
            Ballots = {}, 
            Plaintexts = {}",
            dbg_hash(&self.0.0),self.1,dbg_hash(&self.2.0),dbg_hash(&self.3.0),dbg_hash(&self.4.0),
        )
    }
}

impl std::fmt::Debug for crate::verify::v::Verified {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Verification
            Configuration = {}, 
            Batch = {}, 
            Verified constructed PublicKey = {}, 
            Verified ballots PublicKey = {}, 
            Signed Ballots = {}, 
            Decryption target = {}, 
            Decryption proofs with respect to PublicKey = {}, 
            Plaintexts = {}, 
            Mixes = {:?}",
            dbg_hash(&self.0.0),self.1,dbg_hash(&self.2.0),dbg_hash(&self.3.0),dbg_hash(&self.4.0),dbg_hash(&self.5.0),dbg_hash(&self.6.0),dbg_hash(&self.7.0),self.8.0.map(|h| hex::encode(h)[0..10].to_string()),    
        )
    }
}