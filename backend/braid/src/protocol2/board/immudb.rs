// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use immu_board::{Board, BoardMessage};
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;

use crate::protocol2::message::Message;

pub struct ImmudbBoard {
    board: Board,
}

impl From<Message> for BoardMessage {
    fn from(message: Message) -> BoardMessage {
        BoardMessage {
            id: 0,
            created: (instant::now() * 1000f64) as i64,
            statement_timestamp: (message.statement.get_timestamp() * 1000) as i64,
            statement_kind: message.statement.get_kind().to_string(),
            message: message.strand_serialize().unwrap(),
            signer_key: message.signer_key.strand_serialize().unwrap(),
        }
    }
}

impl ImmudbBoard {
    pub async fn new(
        server_url: &str,
        username: &str,
        password: &str,
        index_dbname: String,
        board_dbname: String,
    ) -> Result<ImmudbBoard> {
        let board = Board::new(server_url, username, password, index_dbname, board_dbname).await?;
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

    
    pub async fn post_messages(
        &mut self, messages: Vec<Message>,
    ) -> Result<()>
    {
        let bm: Vec<BoardMessage> = messages.into_iter().map(|m| {
            m.into()
        }).collect();
        self.board.insert_messages(&bm).await
    }

    pub async fn close(&mut self) -> Result<()> {
        self.board.close().await
    }
}
