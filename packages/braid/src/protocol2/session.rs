use crate::protocol2::board::immudb::ImmudbBoard;
use crate::protocol2::trustee::Trustee;
use anyhow::Result;
use strand::context::Ctx;
use tracing::{info, warn};

pub struct Session<C: Ctx + 'static> {
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

    // Takes ownership of self to allow spawning threads in parallel
    // See https://stackoverflow.com/questions/63434977/how-can-i-spawn-asynchronous-methods-in-a-loop
    pub async fn step(mut self) -> Result<Self> {
        let messages = self.board.get_messages(self.last_message_id).await?;

        if 0 == messages.len() {
            info!("No messages in board, no action taken");

            return Ok(self)
        }

        let (send_messages, _actions) = self.trustee.step(messages)?;
        if !self.dry_run {
            let result = self.board.insert_messages(send_messages).await;
            match result {
                Ok(_) => (),
                Err(err) => {
                    warn!("Insert messages returns error {:?}", err)
                },
            }
        }

        Ok(self)
    }
}
