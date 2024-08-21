// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use tracing::info;

use strand::context::Ctx;

use crate::protocol::board::{Board, BoardFactory};
use crate::protocol::trustee::Trustee;
use crate::util::ProtocolError;

pub struct Session<C: Ctx + 'static, B: Board + 'static> {
    pub name: String,
    trustee: Trustee<C>,
    board: B::Factory,
    last_message_id: Option<i64>,
}
impl<C: Ctx, B: Board> Session<C, B> {
    pub fn new(name: &str, trustee: Trustee<C>, board: B::Factory) -> Session<C, B> {
        Session {
            name: name.to_string(),
            trustee,
            board,
            last_message_id: None,
        }
    }

    // Takes ownership of self to allow spawning threads in parallel
    // See https://stackoverflow.com/questions/63434977/how-can-i-spawn-asynchronous-methods-in-a-loop
    // See also protocol_test_grpc::run_protocol_test
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

        if messages.len() == 0 {
            info!("No new messages retrieved, session step finished");
            return (self, Ok(()));
        }

        let step_result = self.trustee.step(messages);
        if let Err(err) = step_result {
            return (self, Err(err));
        }
        let (send_messages, _actions, last_id) = step_result.expect("impossible");

        let result = board
            .insert_messages(send_messages)
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()));

        info!("Setting last_id = {}", last_id);
        self.last_message_id = Some(last_id);

        (self, result)
    }
}
