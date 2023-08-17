// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use immu_board::{Board, BoardMessage};
use strand::serialization::StrandDeserialize;

use crate::protocol2::message::Message;

pub struct ImmudbBoard {
    board: Board,
}


impl ImmudbBoard {
    pub async fn new(
        server_url: &str,
        index_dbname: String,
        board_dbname: String,
    ) -> Result<ImmudbBoard> {
        let board = Board::new(server_url, index_dbname, board_dbname).await?;
        Ok(ImmudbBoard {
            board: board,
        })
    }

    pub async fn get_messages(
        &mut self, last_id: i64
    ) -> Result<Vec<Message>>
    {
        self.board
            .get_messages(last_id)
            .await?
            .iter()
            .map(|board_message: &BoardMessage| {
                Ok(Message::strand_deserialize(&board_message.message)?)
            })
            .collect()
    }

}
