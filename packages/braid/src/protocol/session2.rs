// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use board_messages::grpc::GrpcB3Message;
use tracing::{info, warn};
// Same line printing
use rusqlite::params;
use rusqlite::Connection;
use std::io::Write;
use std::path::PathBuf;
use strand::serialization::StrandDeserialize;

use strand::context::Ctx;

use crate::protocol::board::{Board, BoardFactory};
use crate::protocol::trustee::Trustee;
use crate::util::ProtocolError;
use board_messages::braid::message::Message;

const RETRIEVE_ALL_PERIOD: i64 = 5 * 60;

pub struct Session2<C: Ctx + 'static> {
    pub board_name: String,
    trustee: Trustee<C>,
    last_message_id: i64,
    last_external_message_id: i64,
    store_root: PathBuf,
    step_counter: i64,
}
impl<C: Ctx> Session2<C> {
    pub fn new(board_name: &str, trustee: Trustee<C>, store_root: &PathBuf) -> Session2<C> {
        Session2 {
            board_name: board_name.to_string(),
            trustee,
            last_message_id: -1,
            last_external_message_id: -1,
            store_root: store_root.clone(),
            step_counter: 1,
        }
    }

    // Takes ownership of self to allow spawning threads in parallel
    // See https://stackoverflow.com/questions/63434977/how-can-i-spawn-asynchronous-methods-in-a-loop
    // See also protocol_test_grpc::run_protocol_test
    // #[instrument(skip_all)]
    pub fn step(
        &mut self,
        messages: &Vec<GrpcB3Message>,
        step_counter: u64,
    ) -> Result<Vec<Message>, ProtocolError> {
        let messages = self
            .store_and_return_messages(messages)
            .map_err(|e| ProtocolError::BoardError(e.to_string()));

        if let Err(err) = messages {
            return Err(err);
        }
        let messages = messages.expect("impossible");

        if messages.len() == 0 {
            /* info!(
                "No new messages retrieved, session step finished ({}, {})",
                self.active_period, step_counter
            );*/
            print!("_");
            let _ = std::io::stdout().flush();
            return Ok(vec![]);
        }

        let step_result = self.trustee.step(messages);
        if let Err(err) = step_result {
            return Err(err);
        }
        let (send_messages, _actions, last_id) = step_result.expect("impossible");
        // last_id is the larget message id that was successfully updated to the trustee's board in memory
        // in the event that there are holes, a session reset will eventually load missing messages
        // from the store
        self.last_message_id = last_id;

        info!("Returning {} messages..", send_messages.len());

        Ok(send_messages)
    }

    // Returns the largest id stored in the local message store
    // in the event that there are holes, an external_last_id reset will eventually load missing
    // messages from the remote board
    pub fn get_last_external_id(&mut self) -> Result<i64> {
        let connection = self.get_store()?;

        let external_last_id =
            connection.query_row("SELECT max(external_id) FROM messages;", [], |row| {
                row.get(0)
            });

        if external_last_id.is_err() {
            warn!(
                "sql error retrieving external_last_id {:?}",
                external_last_id
            );
        }

        self.step_counter += 1;
        let reset = self.step_counter % RETRIEVE_ALL_PERIOD == 0;
        let external_last_id = if reset {
            -1
        } else {
            external_last_id.unwrap_or(-1)
        };

        Ok(external_last_id)
    }

    fn store_and_return_messages(
        &mut self,
        messages: &Vec<GrpcB3Message>,
    ) -> Result<Vec<(Message, i64)>> {
        let connection = self.get_store()?;

        let reset = self.step_counter % RETRIEVE_ALL_PERIOD == 0;
        // FIXME verify message signatures before inserting in local store
        let mut statement = if reset {
            connection.prepare(
                "INSERT OR IGNORE INTO MESSAGES(external_id, message, blob_hash) VALUES(?1, ?2, ?3)",
            )?
        } else {
            connection.prepare(
                "INSERT INTO MESSAGES(external_id, message, blob_hash) VALUES(?1, ?2, ?3)",
            )?
        };

        connection.execute("BEGIN TRANSACTION", [])?;
        for m in messages {
            let hash = strand::hash::hash(&m.message)?;
            statement.execute(params![m.id, m.message, hash])?;
        }
        connection.execute("END TRANSACTION", [])?;

        let mut stmt =
            connection.prepare("SELECT id,message FROM MESSAGES where id > ?1 order by id asc")?;

        let rows = stmt.query_map([self.last_message_id], |row| {
            Ok(MessageRow {
                id: row.get(0)?,
                message: row.get(1)?,
            })
        })?;

        let messages: Result<Vec<(Message, i64)>> = rows
            .map(|mr| {
                let row = mr?;
                let id = row.id;
                let message = Message::strand_deserialize(&row.message)?;
                Ok((message, id))
            })
            .collect();

        messages
    }

    fn get_store(&self) -> Result<Connection> {
        let db_path = self.store_root.join(&self.board_name);
        let connection = Connection::open(&db_path)?;
        // The autogenerated id column is used to establish an order that cannot be manipulated by the external board. Once a retrieved message is
        // stored and assigned a local id, it is not possible for later messages to have an earlier id.
        // The external_id column is used to retrieve _new_ messages as defined by the external board (to optimize bandwidth).
        connection.execute("CREATE TABLE if not exists MESSAGES(id INTEGER PRIMARY KEY AUTOINCREMENT, external_id INT8 NOT NULL UNIQUE, message BLOB NOT NULL, blob_hash BLOB NOT NULL UNIQUE)", [])?;

        Ok(connection)
    }
}

struct MessageRow {
    id: i64,
    message: Vec<u8>,
}
