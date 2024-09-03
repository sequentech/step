// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Result, anyhow};
use board_messages::grpc::GrpcB3Message;
use rusqlite::params;
use rusqlite::Connection;
use strand::signature::StrandSignatureSk;
use strand::symm::SymmetricKey;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::sync::mpsc::Receiver;
use strand::serialization::StrandDeserialize;
use std::collections::HashMap;
use strand::backend::ristretto::RistrettoCtx;
use tokio::sync::mpsc;

use strand::context::Ctx;

use tokio::time::{sleep, Duration};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use crate::protocol::board::grpc2::{
    BoardFactoryMulti, BoardMulti, GrpcB3BoardParams,
};
use crate::protocol::trustee::Trustee;
use crate::util::ProtocolError;
use board_messages::braid::message::Message;

use super::trustee::TrusteeConfig;

const RETRIEVE_ALL_PERIOD: i64 = 5 * 60;
const SESSION_RESET_PERIOD: i64 = 5 * 60;

pub struct Session2<C: Ctx + 'static> {
    pub board_name: String,
    trustee: Trustee<C>,
    // last message retrieved from message_Store
    last_message_id: i64,
    store_root: PathBuf,
    step_counter: i64,
}
impl<C: Ctx> Session2<C> {
    pub fn new(board_name: &str, trustee: Trustee<C>, store_root: &PathBuf) -> Result<Session2<C>> {
        let ret = Session2 {
            board_name: board_name.to_string(),
            trustee,
            last_message_id: -1,
            store_root: store_root.clone(),
            step_counter: 1,
        };

        // fail early
        ret.get_store()?;

        Ok(ret)
    }

    pub fn step(
        &mut self,
        messages: &Vec<GrpcB3Message>,
    ) -> Result<Vec<Message>, ProtocolError> {
        let messages = self
            .store_and_return_messages(messages)
            .map_err(|e| ProtocolError::BoardError(e.to_string()))?;

        // NOTE: we must call step even if there are no new remote messages
        // because there may be actions pending in the trustees memory board
        let step_result = self.trustee.step(messages);
        if let Err(err) = step_result {
            return Err(err);
        }
        // let (send_messages, _actions, last_id) = step_result.expect("impossible");
        let step_result = step_result.expect("impossible");
        // last_id is the largest message id that was successfully updated to the trustee's board in memory
        // in the event that there are holes, a session reset will eventually load missing messages
        // from the store
        if step_result.added_messages > 0 {
            // if no messages were
            self.last_message_id = step_result.last_id;
        }

        Ok(step_result.messages)
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

pub enum SessionSetMessage {
    REFRESH(Vec<String>),
}

pub struct SessionSet {
    name: String,
    session_factory: SessionFactory,
    b3_url: String,
    inbox: Receiver<SessionSetMessage>
}


impl SessionSet {
    pub fn new(name: &str, session_factory: &SessionFactory, b3_url: &str, inbox: mpsc::Receiver<SessionSetMessage>) -> Result<Self> {
        Ok(SessionSet {
            name: name.to_string(),
            session_factory: session_factory.clone(),
            b3_url: b3_url.to_string(),
            inbox
        })
    }
    
    pub fn run(mut self) -> JoinHandle<()> {
        let handler = tokio::spawn(async move {
            
            let mut session_map: HashMap<String, Session2<RistrettoCtx>> = HashMap::new();
            let mut loop_count: i64 = 0;
            
            loop {
                loop_count += 1;
                let signal = self.inbox.recv().await;

                if signal.is_none() {
                    warn!("Set {}: shutting down ({})..", self.name, loop_count);
                    break;
                }
                
                match signal.expect("impossible") {
                    SessionSetMessage::REFRESH(boards) => {
                        // info!("Set {}: ({}) received refresh for {} boards", self.name, session_map.len(), boards.len());
                        for b in boards.iter() {
                            if !session_map.contains_key(b) {
                                info!("Set {}: adding session '{}'..", self.name, b);
                                let session = self.session_factory.get_session(b);
                                if let Ok(session) = session {
                                    session_map.insert(b.to_string(), session);
                                }
                                else {
                                    error!("Unable to create session '{}': {} (set {})", b, session.err().unwrap(), self.name);
                                }
                            }
                        }
                        
                        let boards_set: HashSet<String> = HashSet::from_iter(boards.into_iter());

                        session_map.retain(|k, _v| {
                            let ret = boards_set.contains(k);
                            if !ret {
                                info!("Set {}: Removing session '{}'", k, self.name);
                            }
                            ret
                        });
                    }
                }

                if loop_count % SESSION_RESET_PERIOD == 0 {
                    info!("* Set {}: Session memory reset: reload all artifacts from store", self.name);
                    let new_sessions: Result<Vec<(String, Session2<RistrettoCtx>)>> = 
                        session_map.keys().map(|k| Ok((k.clone(), self.session_factory.get_session(&k)?)))
                        .collect();

                    if let Ok(new_sessions) = new_sessions {
                        session_map = new_sessions.into_iter().collect();
                    }
                    else {
                        error!("Unable to reset sessions: {:?}", new_sessions.err().unwrap());
                    }
                }

                let mut requests: Vec<(String, i64)> = vec![];
                for session in session_map.values_mut() {
                    let last_id = session.get_last_external_id();
                    
                    let Ok(last_id) = last_id else {
                        warn!(
                            "sql error retrieving external_last_id {:?}",
                            last_id
                        );
                        continue;
                    };

                    // info!("Set {}: board {}, external_last_id: {}", self.name, session.board_name, last_id);
                    requests.push((session.board_name.to_string(), last_id));
                }
                info!("Set {}: gathered {} requests", self.name, requests.len());
                
                let board = GrpcB3BoardParams::new(&self.b3_url);
                let board = board.get_board();
                let responses = board.get_messages_multi(&requests).await;
                let Ok(responses) = responses else {
                    error!(
                        "Error retrieving messages for {} requests: {} (set {})",
                        requests.len(),
                        responses.err().unwrap(),
                        self.name
                    );
                    sleep(Duration::from_millis(1000)).await;
                    continue;
                };
                info!("Set {}: received {} keyed messages", self.name, responses.len());

                let mut post_messages = vec![];
                let mut total_bytes: u32 = 0;

                
                let tuples = responses.into_iter()
                    .map(|km| (km.board, km.messages));
                let km_table: HashMap<String, Vec<GrpcB3Message>> = HashMap::from_iter(tuples);

                for (k, s) in session_map.iter_mut() {
                    let empty = vec![];
                    let messages = km_table.get(k).unwrap_or(&empty);
                    // NOTE: we must call step even if there are no new remote messages
                    // because there may be messages pending in the message_store
                    let messages = s.step(messages);
        
                    let Ok(messages) = messages else {
                        let _ = messages.inspect_err(|error| {
                            error!(
                                "Error executing step for board '{}': '{:?}' (set {})",
                                k, error, self.name
                            );
                        });
        
                        continue;
                    };
        
                    if messages.len() > 0 {
                        let next_bytes: usize = messages
                            .iter()
                            .map(|m| m.artifact.as_ref().map(|v| v.len()).unwrap_or(0))
                            .sum();
                        total_bytes += next_bytes as u32;
                        post_messages.push((k.clone(), messages));
                    }
                }
        
                /*for km in responses {
        
                    let s = session_map.get_mut(&km.board);
                    let Some(s) = s else {
                        error!("Could not retrieve session with name: '{}'", km.board);
                        continue;
                    };
                    // println!("Step for {} with {} messages", km.board, km.messages.len());
                    let messages = s.step(&km.messages);
        
                    let Ok(messages) = messages else {
                        let _ = messages.inspect_err(|error| {
                            error!(
                                "Error executing step for board '{}': '{:?}'",
                                km.board, error
                            );
                        });
        
                        continue;
                    };
        
                    if messages.len() > 0 {
                        let next_bytes: usize = messages
                            .iter()
                            .map(|m| m.artifact.as_ref().map(|v| v.len()).unwrap_or(0))
                            .sum();
                        total_bytes += next_bytes as u32;
                        post_messages.push((km.board, messages));
                    }
                }*/

                if post_messages.len() > 0 {
                    info!(
                        "Set {}: posting {} keyed messages with {:.2} MB",
                        self.name,
                        post_messages.len(),
                        f64::from(total_bytes) / (1024.0 * 1024.0)
                    );
                    let result = board.insert_messages_multi(post_messages).await;
                    if let Err(err) = result {
                        error!("Error posting messages: '{:?} (set {})'", err, self.name);
                    }
                } else {
                    info!("No messages to post on this step");
                }
            }
            std::process::exit(1);

        });

        // std::process::exit(1);
        handler
    }
}

#[derive(Clone)]
pub struct SessionFactory {
    trustee_name: String,
    signing_key: StrandSignatureSk,
    symm_key: SymmetricKey,
    store_root: PathBuf,
}
impl SessionFactory {
    pub fn new(trustee_name: &str, cfg: TrusteeConfig, store_root: PathBuf) -> Result<Self> {
        let signing_key: StrandSignatureSk = StrandSignatureSk::from_der_b64_string(&cfg.signing_key_sk)?;

        let bytes = crate::util::decode_base64(&cfg.encryption_key)?;
        let symm_key = strand::symm::sk_from_bytes(&bytes)?;
        
        if !store_root.is_dir() {
            return Err(anyhow!("Invalid store root {:?}", store_root));
        }

        Ok(SessionFactory {
            trustee_name: trustee_name.to_string(),
            symm_key,
            signing_key,
            store_root
        })
    }

    pub fn get_session(&self, board_name: &str) -> Result<Session2<RistrettoCtx>> {
        info!(
            "* Creating new session for board '{}'..",
            board_name
        );

        let trustee: Trustee<RistrettoCtx> = Trustee::new(
            self.trustee_name.clone(),
            self.signing_key.clone(),
            self.symm_key,
        );
        
        Session2::new(board_name, trustee, &self.store_root)
    }
}