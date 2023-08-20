// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use immu_board::{BoardClient, BoardMessage, IndexBoard};
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;

use crate::protocol2::message::Message;

pub struct ImmudbBoard {
    board_client: BoardClient,
    board_dbname: String,
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
        board_dbname: String,
    ) -> Result<ImmudbBoard> {
        let board_client = BoardClient::new(server_url, username, password).await?;
        Ok(ImmudbBoard {
            board_client: board_client,
            board_dbname
        })
    }

    pub async fn get_messages(
        &mut self, last_id: i64
    ) -> Result<Vec<Message>> {
        self.board_client
            .get_messages(&self.board_dbname, last_id)
            .await?
            .iter()
            .map(|board_message: &BoardMessage| {
                Ok(Message::strand_deserialize(&board_message.message)?)
            })
            .collect()
    }
    
    pub async fn post_messages(
        &mut self, messages: Vec<Message>,
    ) -> Result<()> {
        let bm: Vec<BoardMessage> = messages.into_iter().map(|m| {
            m.into()
        }).collect();
        self.board_client.insert_messages(&self.board_dbname, &bm).await
    }
}

pub struct ImmudbBoardIndex {
    board_client: BoardClient,
    index_dbname: String,
}

impl ImmudbBoardIndex {
    pub async fn new(
        server_url: &str,
        username: &str,
        password: &str,
        index_dbname: String,
    ) -> Result<ImmudbBoardIndex> {
        let board_client = BoardClient::new(server_url, username, password).await?;
        Ok(ImmudbBoardIndex {
            board_client: board_client,
            index_dbname
        })
    }

    pub async fn get_board_names(
        &mut self
    ) -> Result<Vec<String>> {
        self.board_client
            .get_boards(&self.index_dbname)
            .await?
            .iter()
            .map(|board: &IndexBoard| {
                Ok(board.database_name.clone())
            })
            .collect()
    }
}

