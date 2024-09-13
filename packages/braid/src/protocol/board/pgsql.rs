// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use board_messages::braid::message::Message;
use board_messages::grpc::pgsql::{B3MessageRow, PgsqlDbConnectionParams, XPgsqlB3Client};
use std::path::PathBuf;

use tracing::{info, warn};

use strand::serialization::StrandDeserialize;

/// A bulletin board implemented on postgresql
pub struct PgsqlBoard {
    pub(crate) board_client: XPgsqlB3Client,
    pub(crate) board_name: String,
}

impl PgsqlBoard {
    pub async fn new(
        connection: &PgsqlDbConnectionParams,
        board_name: String,
    ) -> Result<PgsqlBoard> {
        let board_client = XPgsqlB3Client::new(connection).await?;
        Ok(PgsqlBoard {
            board_client,
            board_name: board_name.to_string(),
        })
    }

    // Returns all messages whose id > last_id.
    async fn get_remote_messages(&mut self, last_id: i64) -> Result<Vec<B3MessageRow>> {
        let messages = self
            .board_client
            .get_messages(&self.board_name, last_id)
            .await?;

        Ok(messages)
    }
}

impl super::Board for PgsqlBoard {
    type Factory = PgsqlBoardParams;

    // Returns all messages whose id > last_id. If last_id is None, all messages will be returned.
    // If a store is used only the messages not previously received will be requested.
    async fn get_messages(&mut self, board: &str, last_id: i64) -> Result<Vec<Message>> {
            
        let messages = self.get_remote_messages(last_id).await?;

        let messages = messages
            .iter()
            .map(|m| {
                let message = Message::strand_deserialize(&m.message)?;
                Ok(message)
            })
            .collect::<Result<Vec<Message>>>()?;
        
        Ok(messages)
    }

    async fn insert_messages(&mut self, board: &str, messages: Vec<Message>) -> Result<()> {
        if messages.len() > 0 {
            let bm: Result<Vec<B3MessageRow>> =
                messages.into_iter().map(|m| m.try_into()).collect();
            self.board_client
                .insert_messages(&self.board_name, &bm?)
                .await
        } else {
            Ok(())
        }
    }
}

pub struct PgsqlBoardParams {
    connection: PgsqlDbConnectionParams,
    board_name: String,
    store_root: Option<PathBuf>,
}
impl PgsqlBoardParams {
    pub fn new(
        connection: &PgsqlDbConnectionParams,
        board_name: String,
        store_root: Option<PathBuf>,
    ) -> PgsqlBoardParams {
        PgsqlBoardParams {
            connection: connection.clone(),
            board_name,
            store_root,
        }
    }
}

impl super::BoardFactory<PgsqlBoard> for PgsqlBoardParams {
    async fn get_board(&self) -> Result<PgsqlBoard> {
        PgsqlBoard::new(
            &self.connection,
            self.board_name.clone(),
            self.store_root.clone(),
        )
        .await
    }
}

struct MessageRow {
    id: i64,
    message: Vec<u8>,
}
