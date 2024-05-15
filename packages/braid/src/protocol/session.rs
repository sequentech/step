// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

use strand::context::Ctx;

use crate::protocol::board::immudb::ImmudbBoard;
use crate::protocol::trustee::Trustee;
use crate::util::ProtocolError;

pub struct Session<C: Ctx + 'static> {
    trustee: Trustee<C>,
    board: BoardParams,
    dry_run: bool,
    last_message_id: Option<i64>,
}
impl<C: Ctx> Session<C> {
    pub fn new(trustee: Trustee<C>, board: BoardParams) -> Session<C> {
        Session {
            trustee,
            board,
            dry_run: false,
            last_message_id: None,
        }
    }

    pub fn new_dry(trustee: Trustee<C>, board: BoardParams) -> Session<C> {
        Session {
            trustee,
            board,
            dry_run: true,
            last_message_id: None,
        }
    }

    // Takes ownership of self to allow spawning threads in parallel
    // See https://stackoverflow.com/questions/63434977/how-can-i-spawn-asynchronous-methods-in-a-loop
    // See also protocol_test_immudb::run_protocol_test_immudb
    pub async fn step(mut self) -> (Self, Result<(), ProtocolError>) {
        let board = self
            .board
            .get_board()
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()));
        if let Err(err) = board {
            return (self, Err(err));
        }
        let mut board = board.expect("impossible");

        let messages = board
            .get_messages(self.last_message_id)
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()));

        if let Err(err) = messages {
            return (self, Err(err));
        }
        let messages = messages.expect("impossible");

        if 0 == messages.len() {
            info!("No messages in board, no action taken");
            return (self, Ok(()));
        }

        // let (send_messages, _actions) = self.trustee.step(messages);
        let step_result = self.trustee.step(messages);
        if let Err(err) = step_result {
            return (self, Err(err));
        }
        let (send_messages, _actions) = step_result.expect("impossible");

        if !self.dry_run {
            let result = board
                .insert_messages(send_messages)
                .await
                .map_err(|e| ProtocolError::BoardError(e.to_string()));
            return (self, result);
            /* match result {
                Ok(_) => (),
                Err(err) => {
                    warn!("Insert messages returns error {:?}", err)
                }
            }*/
        }

        (self, Ok(()))
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
