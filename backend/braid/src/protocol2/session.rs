use crate::protocol2::board::trillian::TrillianBoard;
use crate::protocol2::trustee::Trustee;
use anyhow::Result;
use bulletin_board::client::CacheStore;
use strand::context::Ctx;
use tracing::info;

pub struct Session<C: Ctx, CS: CacheStore> {
    trustee: Trustee<C>,
    board: TrillianBoard<CS>,
}
impl<C: Ctx, CS: CacheStore> Session<C, CS> {
    pub fn new(trustee: Trustee<C>, board: TrillianBoard<CS>) -> Session<C, CS> {
        Session { trustee, board }
    }

    pub async fn step(&mut self) -> Result<()> {
        info!("Trustee {:?} step..", self.trustee.get_pk());

        if let Ok(messages) = self.board.get_messages().await {
            let step_result = self.trustee.step(messages);
            if let Ok((send_messages, _actions)) = step_result {
                let sent = self.board.send_messages(send_messages).await;
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
