use crate::protocol2::action::Message;
use crate::protocol2::datalog::Predicate;
use crate::protocol2::predicate::CiphertextsHash;
use crate::protocol2::statement::StatementType;
use crate::protocol2::trustee::Trustee;
use crate::protocol2::PROTOCOL_MANAGER_INDEX;
use crate::protocol2::{board::immudb::ImmudbBoard, statement::Statement};
use anyhow::Result;
use strand::context::Ctx;
use tracing::{error, info};

pub struct Session<C: Ctx> {
    trustee: Trustee<C>,
    board: ImmudbBoard,
    dry_run: bool,
}
impl<C: Ctx> Session<C> {
    pub fn new(trustee: Trustee<C>, board: ImmudbBoard) -> Session<C> {
        Session {
            trustee,
            board,
            dry_run: false,
        }
    }

    pub fn new_dry(trustee: Trustee<C>, board: ImmudbBoard) -> Session<C> {
        Session {
            trustee,
            board,
            dry_run: true,
        }
    }

    pub async fn step(&mut self) -> Result<()> {
        info!("Trustee {:?} step..", self.trustee.get_pk());

        let messages = self.board.get_messages(0).await?;
        let (send_messages, _actions) = self.trustee.step(messages)?;
        if !self.dry_run {
            self.board.insert_messages(send_messages).await?;
        }

        Ok(())
    }
}

pub struct VerifyingSession<C: Ctx> {
    trustee: Trustee<C>,
    board: ImmudbBoard,
}
impl<C: Ctx> VerifyingSession<C> {
    pub fn new(trustee: Trustee<C>, board: ImmudbBoard) -> VerifyingSession<C> {
        VerifyingSession { trustee, board }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Verifying trustee run..");

        let messages = self.board.get_messages(-1).await?;
        let (messages, mut predicates) = self.trustee.verify(messages)?;
        for message in messages.clone() {
            info!("Verifier produced message {:?}", message);
            let predicate = Predicate::from_statement::<C>(
                &message.statement,
                crate::protocol2::VERIFIER_INDEX,
            );
            predicates.push(predicate);
        }

        let predicates = crate::protocol2::datalog::v::S.run(&predicates);
        info!("Predicates: {:?}", predicates);

        // let ballots = self.trustee.get_ballots

        let mix_signatures: Vec<Message> = messages
            .clone()
            .into_iter()
            .filter(|m| m.statement.get_kind() == StatementType::MixSigned)
            .collect();

        let first = mix_signatures
            .iter()
            .find(|s| s.statement.get_data().3 == 1)
            .unwrap();
        let ballots_h = match first.statement.clone() {
            Statement::MixSigned(_, _, _, _, b, _) => b,
            _ => panic!(),
        };
        let ballots = self
            .trustee
            .get_ballots(&CiphertextsHash(ballots_h.0), 1, PROTOCOL_MANAGER_INDEX)
            .unwrap();
        // first.statement.

        let pk_signature: Vec<Message> = messages
            .clone()
            .into_iter()
            .filter(|m| m.statement.get_kind() == StatementType::PublicKeySigned)
            .collect();

        let plaintext_signature: Vec<Message> = messages
            .into_iter()
            .filter(|m| m.statement.get_kind() == StatementType::PlaintextsSigned)
            .collect();

        Ok(())
    }
}
