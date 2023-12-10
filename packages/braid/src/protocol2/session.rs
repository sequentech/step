use crate::protocol2::board::immudb::ImmudbBoard;
use crate::protocol2::trustee::Trustee;
use anyhow::Result;
use strand::context::Ctx;
use std::path::PathBuf;
use tracing::info;

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
        let messages = self.board.get_messages(self.last_message_id).await?;

        if 0 == messages.len() {
            info!("No messages in board, no action taken");

            return Ok(())
        }

        let (send_messages, _actions) = self.trustee.step(messages)?;
        if !self.dry_run {
            self.board.insert_messages(send_messages).await?;
        }

        Ok(())
    }
}

pub struct BoardParams {
    server_url: String,
    user: String,
    password: String,
    board_name: String,
    store_root: PathBuf,
}
impl BoardParams {
    
    pub fn new(
        server_url: &str,
        user: &str,
        password: &str,
        board_dbname: String,
        store_root: PathBuf,
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
            &self.board_name,
            self.store_root.clone(),
        ).await
    }
}