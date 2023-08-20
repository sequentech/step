use crate::protocol2::board::immudb::ImmudbBoard;
use crate::protocol2::trustee::Trustee;
use anyhow::Result;
use strand::context::Ctx;
use tracing::info;

pub struct Session<C: Ctx> {
    trustee: Trustee<C>,
    board: ImmudbBoard,
}
impl<C: Ctx> Session<C> {
    pub fn new(trustee: Trustee<C>, board: ImmudbBoard) -> Session<C> {
        Session { trustee, board }
    }

    pub async fn step(&mut self) -> Result<()> {
        info!("Trustee {:?} step..", self.trustee.get_pk());

        if let Ok(messages) = self.board.get_messages(0).await {
            let step_result = self.trustee.step(messages);
            if let Ok((send_messages, _actions)) = step_result {
                let sent = self.board.post_messages(send_messages).await;
                if sent.is_err() {
                    info!("Could not send messages");
                }
            } else {
                info!("Step returns error {:?}", step_result);
            }
        } else {
            info!("Could not retrieve messages");
        }

        Ok(())
    }
}
