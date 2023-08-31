use crate::protocol2::action::Message;
use crate::protocol2::artifact::Configuration;
use crate::protocol2::board::immudb::ImmudbBoard;
use crate::protocol2::datalog::{
    BatchNumber, ConfigurationHash, MixingHashes, PlaintextsHash, Predicate,
};
use crate::protocol2::predicate::CiphertextsHash;
use crate::protocol2::statement::{StatementType, Statement, CiphertextsH, PlaintextsH, Batch};
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

        let plaintexts: Vec<Message> = messages
            .clone()
            .into_iter()
            .filter(|m| m.statement.get_kind() == StatementType::Plaintexts)
            .collect();

        let pl_data: Vec<(Batch, PlaintextsH, CiphertextsH)> = plaintexts.into_iter().map(|p| {
            let (batch, pl_h, ballots_h) = match p.statement {
                Statement::Plaintexts(ts, cfg, b, pl_h, dhs, b_h) => (b, pl_h, b_h),
                _ => panic!(),
            };

            (batch, pl_h, ballots_h)
        }).collect();
        

        let pk: Vec<Message> = messages
            .clone()
            .into_iter()
            .filter(|m| m.statement.get_kind() == StatementType::PublicKey)
            .collect();

        let (messages, mut predicates) = self.trustee.verify(messages)?;

        for message in messages.clone() {
            info!("Verifier produced message {:?}", message);
            let predicate = Predicate::from_statement::<C>(
                &message.statement,
                crate::protocol2::VERIFIER_INDEX,
                &cfg,
            );
            predicates.push(predicate);
        }

        let predicates = crate::protocol2::datalog::v::S.run(&predicates);

        let zs: Result<
            Vec<(
                ConfigurationHash,
                BatchNumber,
                CiphertextsHash,
                PlaintextsHash,
                MixingHashes,
            )>,
        > = predicates
            .iter()
            .map(|p| match p {
                Predicate::Z(cfg, batch, ballots, plaintexts, mixes) => {
                    Ok((*cfg, *batch, *ballots, *plaintexts, *mixes))
                }
                _ => Err(anyhow!("Unexpected predicate type")),
            })
            .collect();
        let chains = zs?;

        let chain = chains.iter().find(|z| z.1 == 1);
        info!("chain: {:?}", chain);

        /*for plaintexts in plaintexts_signatures {
            match plaintexts {
                Statement::PlaintextsSigned(ts, cfg, batch, pl_h, _, _) => (cfg, batch, pl_h),
                _ => Err()
            }
        }*/

        Ok(())
    }
}