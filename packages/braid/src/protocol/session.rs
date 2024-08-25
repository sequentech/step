// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use tracing::{info, instrument, warn};

use strand::context::Ctx;

use crate::protocol::board::{Board, BoardFactory};
use crate::protocol::trustee::Trustee;
use crate::util::ProtocolError;

pub struct Session<C: Ctx + 'static, B: Board + 'static> {
    pub name: String,
    trustee: Trustee<C>,
    board: B::Factory,
    last_message_id: Option<i64>,
    active_period: u64,
}
impl<C: Ctx, B: Board> Session<C, B> {
    pub fn new(name: &str, trustee: Trustee<C>, board: B::Factory) -> Session<C, B> {
        Session {
            name: name.to_string(),
            trustee,
            board,
            last_message_id: None,
            active_period: 1,
        }
    }

    // Takes ownership of self to allow spawning threads in parallel
    // See https://stackoverflow.com/questions/63434977/how-can-i-spawn-asynchronous-methods-in-a-loop
    // See also protocol_test_grpc::run_protocol_test
    #[instrument(skip_all)]
    pub async fn step(mut self, step_counter: u64) -> (Self, Result<(), ProtocolError>) {
        // Never skip more than 20 steps
        if self.active_period > 21 {
            self.active_period = 21;
        }

        // Skips steps depending on how active we are
        if (step_counter % self.active_period) != 0 {
            return (self, Ok(()));
        }

        let board = self
            .board
            .get_board()
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()));

        let Ok(mut board) = board else {
            // Surely there's a better way to do this. And don't call me Shirley.
            return (self, board.map(|_| ()));
        };
        /* if let Err(err) = board {
            return (self, Err(err));
        }
        let mut board = board.expect("impossible");*/

        let messages = board
            .get_messages(self.last_message_id)
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()));

        if let Err(err) = messages {
            return (self, Err(err));
        }
        let messages = messages.expect("impossible");

        if messages.len() == 0 {
            info!(
                "No new messages retrieved, session step finished ({}, {})",
                self.active_period, step_counter
            );
            self.active_period = self.active_period * 2;
            return (self, Ok(()));
        }

        let step_result = self.trustee.step(messages);
        if let Err(err) = step_result {
            return (self, Err(err));
        }
        let (send_messages, _actions, last_id) = step_result.expect("impossible");

        if send_messages.len() > 0 {
            self.active_period = 1;
        }

        info!("Posting {} messages..", send_messages.len());

        let result = board
            .insert_messages(send_messages)
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()));

        if result.is_ok() {
            info!("Setting last_id = {}", last_id);
            self.last_message_id = Some(last_id);
        } else {
            warn!(
                "Error posting messages, last_id remains at: {:?}",
                self.last_message_id
            );
        }

        (self, result)
    }
}
