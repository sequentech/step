// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use tracing::info;

use strand::context::Ctx;

use crate::protocol::board::{Board, BoardFactory};
use crate::protocol::trustee2::Trustee;
use crate::util::ProtocolError;

/// A protocol session.
///
/// A protocol session handles one board in the
/// bulletin board.
pub struct Session<C: Ctx + 'static, B: Board + 'static> {
    pub board_name: String,
    trustee: Trustee<C>,
    board_factory: B::Factory,
}
impl<C: Ctx, B: Board> Session<C, B> {
    /// Constructs a new SessionM to handle the requested board.
    ///
    /// The board_factory parameter is used at each step to perform
    /// messaging to/from the remote bulletin board.
    pub fn new(board_name: &str, trustee: Trustee<C>, board_factory: B::Factory) -> Session<C, B> {
        Session {
            board_name: board_name.to_string(),
            trustee,
            board_factory,
        }
    }

    /// Performs one step of the protocol for this session.
    ///
    /// A step performs the following operations
    ///
    /// 1) Retrieve new messages from the remote board (as per
    /// trustee::get_last_external_id)
    /// 2) Run the trustee step
    /// 3) Post the messages returned by the trustee
    /// to the remote board
    pub async fn step(&mut self) -> Result<(), ProtocolError> {
        let mut board = self.board_factory.get_board();

        let external_last_id = self.trustee.get_last_external_id()?;

        let messages = board
            .get_messages(&self.board_name, external_last_id)
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()))?;

        // NOTE: we must call step even if there are no new remote messages
        // because there may be actions pending in the trustee's LocalBoard.
        let step_result = catch_step_panic(|| self.trustee.step(&messages))?;

        info!("Posting {} messages..", step_result.messages.len());

        let result = board
            .insert_messages(&self.board_name, step_result.messages)
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()));

        result
    }
}

/// Runs one trustee step, turning a panic into an error for this board so
/// that it does not stop the steps of the other boards.
fn catch_step_panic<T>(
    step: impl FnOnce() -> Result<T, ProtocolError>,
) -> Result<T, ProtocolError> {
    panic::catch_unwind(AssertUnwindSafe(step)).unwrap_or_else(|payload| {
        Err(ProtocolError::InternalError(format!(
            "Trustee step panicked: {}",
            panic_message(payload.as_ref())
        )))
    })
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    if let Some(message) = payload.downcast_ref::<&str>() {
        message
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message
    } else {
        "non-string panic payload"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catch_step_panic_returns_step_result() {
        let result = catch_step_panic(|| Ok(1));

        assert!(matches!(result, Ok(1)));
    }

    #[test]
    fn catch_step_panic_returns_step_error() {
        let result: Result<(), ProtocolError> =
            catch_step_panic(|| Err(ProtocolError::BoardError("board unavailable".to_string())));

        assert!(matches!(result, Err(ProtocolError::BoardError(_))));
    }

    #[test]
    fn catch_step_panic_returns_internal_error_when_step_panics() {
        let result: Result<(), ProtocolError> = catch_step_panic(|| panic!("step failed"));

        assert!(
            matches!(result, Err(ProtocolError::InternalError(ref m)) if m.contains("step failed"))
        );
    }
}
