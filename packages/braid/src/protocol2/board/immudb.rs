// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use braid_messages::statement::StatementType;
use immu_board::{Board, BoardClient, BoardMessage};
use rusqlite::params;
use rusqlite::Connection;
use std::path::PathBuf;
use strand::serialization::StrandDeserialize;
// use strand::serialization::StrandSerialize;
use braid_messages::message::Message;
use tokio::sync::Mutex;

pub struct ImmudbBoard {
    pub(crate) board_client: BoardClient,
    pub(crate) board_dbname: String,
    pub(crate) store_root: PathBuf,
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
        let connection_mutex = Mutex::new(self.get_store()?);
        let mut channels = self.update_store(&connection_mutex).await?;
        let connection = connection_mutex.lock().await;
        let mut messages = self.get_store_messages(&connection, last_id)?;

        /*
        connection
            .close()
            .map_err(|e| anyhow!("Could not close sqlite connection: {:?}", e))?;
        */
        // Allows for Channel deletion
        messages.append(&mut channels);

        Ok(messages)
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

    async fn update_store(&mut self, connection_mutex: &Mutex<Connection>) -> Result<Vec<Message>> {
        let connection = connection_mutex.lock().await;
        let mut channel_messages = vec![];

        let last_id: Result<i64> = connection
            .query_row("SELECT max(id) FROM messages;", [], |row| row.get(0))
            .or(Ok(0i64));
        let last_id = last_id?;

        let messages = self
            .board_client
            .get_messages(&self.board_dbname, last_id)
            .await?;

        let mut monotonic = -1;
        for message in messages {
            // new messages must always be appended to the store in ascending order
            assert!(message.id > last_id);
            if monotonic != -1 {
                assert_eq!(message.id, monotonic + 1);
                monotonic = message.id;
            }

            // Allows for Channel deletion
            let m = Message::strand_deserialize(&message.message)?;
            if m.statement.get_kind() == StatementType::Channel {
                channel_messages.push(m);
            } else {
                connection.execute(
                    "INSERT INTO MESSAGES VALUES(?1, ?2)",
                    params![message.id, message.message],
                )?;
            }
        }

        Ok(channel_messages)
    }

    fn get_store_messages(
        &mut self,
        connection: &Connection,
        last_id: i64,
    ) -> Result<Vec<Message>> {
        let mut stmt =
            connection.prepare("SELECT id,message FROM MESSAGES where id > ?1 order by id asc")?;
        let rows = stmt.query_map([last_id], |row| {
            Ok(MessageRow {
                _id: row.get(0)?,
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
    _id: u64,
    message: Vec<u8>,
}
