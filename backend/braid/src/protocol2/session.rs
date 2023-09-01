use crate::protocol2::board::immudb::ImmudbBoard;
use crate::protocol2::trustee::Trustee;
use anyhow::{anyhow, Result};
use strand::context::Ctx;
use tracing::{error, info};

pub struct Session<C: Ctx> {
    trustee: Trustee<C>,
    board: ImmudbBoard,
    dry_run: bool,
    last_message_id: i64,
}
impl<C: Ctx> Session<C> {
    pub fn new(trustee: Trustee<C>, board: ImmudbBoard) -> Session<C> {
        Session {
            trustee,
            board,
            dry_run: false,
            last_message_id: -1,
        }
    }

    pub fn new_dry(trustee: Trustee<C>, board: ImmudbBoard) -> Session<C> {
        Session {
            trustee,
            board,
            dry_run: true,
            last_message_id: -1,
        }
    }

    pub async fn step(&mut self) -> Result<()> {
        info!("Trustee {:?} step..", self.trustee.get_pk());

        let messages = self.board.get_messages(self.last_message_id).await?;
        let (send_messages, _actions) = self.trustee.step(messages)?;
        if !self.dry_run {
            self.board.insert_messages(send_messages).await?;
        }

        Ok(())
    }
}
