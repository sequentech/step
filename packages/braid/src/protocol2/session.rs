use crate::protocol2::board::immudb::ImmudbBoard;
use crate::protocol2::trustee::Trustee;
use anyhow::Result;
use std::path::PathBuf;
use strand::context::Ctx;
use tracing::{info, warn};

pub struct Session<C: Ctx + 'static> {
    trustee: Trustee<C>,
    board: BoardParams,
    dry_run: bool,
    last_message_id: i64,
}
impl<C: Ctx> Session<C> {
    pub fn new(trustee: Trustee<C>, board: BoardParams) -> Session<C> {
        Session {
            trustee,
            board,
            dry_run: false,
            last_message_id: -1,
        }
    }

    pub fn new_dry(trustee: Trustee<C>, board: BoardParams) -> Session<C> {
        Session {
            trustee,
            board,
            dry_run: true,
            last_message_id: -1,
        }
    }

    // Takes ownership of self to allow spawning threads in parallel
    // See https://stackoverflow.com/questions/63434977/how-can-i-spawn-asynchronous-methods-in-a-loop
    // See also protocol_test_immudb::run_protocol_test_immudb
    pub async fn step(mut self) -> Result<Self> {
        let mut board = self.board.get_board().await?;
        let messages = board.get_messages(self.last_message_id).await?;

        if 0 == messages.len() {
            info!("No messages in board, no action taken");
            return Ok(self);
        }

        let (send_messages, _actions) = self.trustee.step(messages)?;
        if !self.dry_run {
            let result = board.insert_messages(send_messages).await;
            match result {
                Ok(_) => (),
                Err(err) => {
                    warn!("Insert messages returns error {:?}", err)
                }
            }
        }

        Ok(self)
    }
}

pub struct BoardParams {
    server_url: String,
    user: String,
    password: String,
    board_name: String,
    store_root: Option<PathBuf>,
}
impl BoardParams {
    pub fn new(
        server_url: &str,
        user: &str,
        password: &str,
        board_dbname: &str,
        store_root: Option<PathBuf>,
    ) -> BoardParams {
        BoardParams {
            server_url: server_url.to_string(),
            user: user.to_string(),
            password: password.to_string(),
            board_name: board_dbname.to_string(),
            store_root: store_root,
        }
    }

    pub async fn get_board(&self) -> Result<ImmudbBoard> {
        ImmudbBoard::new(
            &self.server_url,
            &self.user,
            &self.password,
            self.board_name.to_string(),
            self.store_root.clone(),
        )
        .await
    }
}
