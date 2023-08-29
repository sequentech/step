// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use immu_board::{Board, BoardClient, BoardMessage};
use rusqlite::params;
use rusqlite::Connection;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;

use crate::protocol2::message::Message;

pub struct ImmudbBoard {
    board_client: BoardClient,
    board_dbname: String,
    store_root: PathBuf,
}

impl TryFrom<Message> for BoardMessage {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<BoardMessage> {
        Ok(BoardMessage {
            id: 0,
            created: (instant::now() * 1000f64) as i64,
            statement_timestamp: (message.statement.get_timestamp() * 1000) as i64,
            statement_kind: message.statement.get_kind().to_string(),
            message: message.strand_serialize()?,
            signer_key: message.signer_key.strand_serialize()?,
        })
    }
}

impl ImmudbBoard {
    pub async fn new(
        server_url: &str,
        username: &str,
        password: &str,
        board_dbname: String,
        store_root: PathBuf,
    ) -> Result<ImmudbBoard> {
        let board_client = BoardClient::new(server_url, username, password).await?;
        Ok(ImmudbBoard {
            board_client: board_client,
            board_dbname,
            store_root,
        })
    }

    pub async fn get_messages(&mut self, last_id: i64) -> Result<Vec<Message>> {
        let connection = self.get_store()?;
        self.update_store(&connection).await?;
        let messages = self.get_store_messages(&connection, last_id);
        connection
            .close()
            .map_err(|e| anyhow!("Could not close sqlite connection: {:?}", e))?;
        messages
    }

    pub async fn insert_messages(&mut self, messages: Vec<Message>) -> Result<()> {
        if messages.len() > 0 {
            let bm: Result<Vec<BoardMessage>> =
                messages.into_iter().map(|m| m.try_into()).collect();
            self.board_client
                .insert_messages(&self.board_dbname, &bm?)
                .await
        } else {
            Ok(())
        }
    }

    fn get_store(&self) -> Result<Connection> {
        let db_path = self.store_root.join(&self.board_dbname);
        let connection = Connection::open(&db_path)?;
        connection.execute("CREATE TABLE if not exists MESSAGES(id INT PRIMAREY KEY, message BLOB NOT NULL UNIQUE)", [])?;

        Ok(connection)
    }

    async fn update_store(&mut self, connection: &Connection) -> Result<()> {
        let last_id: Result<i64> = connection
            .query_row("SELECT max(id) FROM messages;", [], |row| row.get(0))
            .or(Ok(0i64));

        let messages = self
            .board_client
            .get_messages(&self.board_dbname, last_id?)
            .await?;

        for message in messages {
            connection.execute(
                "INSERT INTO MESSAGES VALUES(?1, ?2)",
                params![message.id, message.message],
            )?;
        }

        Ok(())
    }

    fn get_store_messages(
        &mut self,
        connection: &Connection,
        last_id: i64,
    ) -> Result<Vec<Message>> {
        let mut stmt = connection.prepare("SELECT id,message FROM MESSAGES where id > ?1")?;
        let rows = stmt.query_map([last_id], |row| {
            Ok(MessageRow {
                id: row.get(0)?,
                message: row.get(1)?,
            })
        })?;

        let messages: Result<Vec<Message>> = rows
            .map(|mr| Ok(Message::strand_deserialize(&mr?.message)?))
            .collect();

        messages
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
            index_dbname,
        })
    }

    pub async fn get_board_names(&mut self) -> Result<Vec<String>> {
        self.board_client
            .get_boards(&self.index_dbname)
            .await?
            .iter()
            .map(|board: &Board| Ok(board.database_name.clone()))
            .collect()
    }
}

struct MessageRow {
    id: u64,
    message: Vec<u8>,
}
