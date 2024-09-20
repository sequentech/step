// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use board_messages::grpc::GrpcB3Message;
use std::collections::HashSet;
use std::path::PathBuf;
use strand::signature::StrandSignatureSk;
use strand::symm::SymmetricKey;
use tokio::sync::mpsc::Receiver;

use std::collections::HashMap;
use strand::backend::ristretto::RistrettoCtx;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

use crate::protocol::board::grpc_m::{BoardFactoryMulti, BoardMulti, GrpcB3BoardParams};
use crate::protocol::trustee2::Trustee;
use crate::protocol::trustee2::TrusteeConfig;
use crate::util::ProtocolError;
use board_messages::braid::message::Message;
use strand::context::Ctx;

// How often the session map (with trustee's memory board) is cleared
const SESSION_RESET_PERIOD: i64 = 20 * 60;

pub struct SessionM<C: Ctx + 'static> {
    pub board_name: String,
    trustee: Trustee<C>,
}
impl<C: Ctx> SessionM<C> {
    pub fn new(board_name: &str, trustee: Trustee<C>) -> Result<SessionM<C>> {
        let ret = SessionM {
            board_name: board_name.to_string(),
            trustee,
        };

        Ok(ret)
    }

    pub fn step(&mut self, messages: &Vec<GrpcB3Message>) -> Result<Vec<Message>, ProtocolError> {
        // NOTE: we must call step even if there are no new remote messages
        // because there may be actions pending in the trustees memory board
        let step_result = self.trustee.step(messages)?;

        Ok(step_result.messages)
    }

    pub fn get_last_external_id(&mut self) -> Result<i64, ProtocolError> {
        self.trustee.get_last_external_id()
    }

    pub(crate) fn update_store(
        &self,
        messages: &Vec<GrpcB3Message>,
    ) -> Result<(), ProtocolError> {
        self.trustee.update_store(messages)
    }
}

pub enum SessionSetMessage {
    REFRESH(Vec<String>),
}

pub struct SessionSet {
    name: String,
    session_factory: SessionFactory,
    b3_url: String,
    inbox: Receiver<SessionSetMessage>,
}
impl SessionSet {
    pub fn new(
        name: &str,
        session_factory: &SessionFactory,
        b3_url: &str,
        inbox: mpsc::Receiver<SessionSetMessage>,
    ) -> Result<Self> {
        Ok(SessionSet {
            name: name.to_string(),
            session_factory: session_factory.clone(),
            b3_url: b3_url.to_string(),
            inbox,
        })
    }

    pub fn run(mut self) -> JoinHandle<()> {
        let handler = tokio::spawn(async move {
            let mut session_map: HashMap<String, SessionM<RistrettoCtx>> = HashMap::new();
            let mut loop_count: i64 = 0;

            loop {
                loop_count = (loop_count + 1) % i64::MAX;
                sleep(Duration::from_millis(1000)).await;
                let signal = self.inbox.try_recv();

                print!(".");

                match signal {
                    Ok(SessionSetMessage::REFRESH(boards)) => {
                        // info!("Set {}: ({}) received refresh for {} boards", self.name, session_map.len(), boards.len());
                        for b in boards.iter() {
                            if !session_map.contains_key(b) {
                                info!("Set {}: adding session '{}'..", self.name, b);
                                let session = self.session_factory.create_session(b);
                                if let Ok(session) = session {
                                    session_map.insert(b.to_string(), session);
                                } else {
                                    error!(
                                        "Unable to create session '{}': {} (set {})",
                                        b,
                                        session.err().unwrap(),
                                        self.name
                                    );
                                }
                            }
                        }

                        let boards_set: HashSet<String> = HashSet::from_iter(boards.into_iter());

                        session_map.retain(|k, _v| {
                            let ret = boards_set.contains(k);
                            if !ret {
                                info!("Set {}: Removing session '{}'", self.name, k);
                            }
                            ret
                        });
                    }
                    // We're polling with try_recv, so ok
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        warn!("Set {}: shutting down ({})..", self.name, loop_count);
                        break;
                    }
                }

                if loop_count % SESSION_RESET_PERIOD == 0 {
                    info!(
                        "* Set {}: Session memory reset: reload all artifacts from store",
                        self.name
                    );
                    let new_sessions: Result<Vec<(String, SessionM<RistrettoCtx>)>> = session_map
                        .keys()
                        .map(|k| Ok((k.clone(), self.session_factory.create_session(&k)?)))
                        .collect();

                    if let Ok(new_sessions) = new_sessions {
                        session_map = new_sessions.into_iter().collect();
                    } else {
                        error!(
                            "Unable to reset sessions: {:?}",
                            new_sessions.err().unwrap()
                        );
                    }
                }

                let mut requests: Vec<(String, i64)> = vec![];
                for session in session_map.values_mut() {
                    let last_id = session.get_last_external_id();

                    let Ok(last_id) = last_id else {
                        warn!("sql error retrieving external_last_id {:?}", last_id);
                        continue;
                    };

                    requests.push((session.board_name.to_string(), last_id));
                }

                /*
                Use this block for load testing
                // dkg messages = 1 + 5n
                // tally messages = b * (n + (t * t + 1) + 1)
                // threshold 3: 32 messages
                let trustees = 3;
                let threshold = 2;
                let dkg_messages = 1 + 5 * trustees;
                let tally_messages = trustees + (threshold * (threshold + 1)) + 1;

                if (loop_count > 5)
                    && (requests[0].1 == dkg_messages
                        || requests[0].1 == (dkg_messages + tally_messages))
                {
                    println!("*** Remove this code!");
                    std::process::exit(0);
                }*/

                let board = GrpcB3BoardParams::new(&self.b3_url);
                let board = board.get_board();
                let responses = board.get_messages_multi(&requests).await;

                // If the bulletin board returns truncated = true it means there
                // are more messages pending that were cut off to not
                // exceed configured message size limit
                let Ok((responses, truncated)) = responses else {
                    error!(
                        "Error retrieving messages for {} requests: {} (set {})",
                        requests.len(),
                        responses.err().unwrap(),
                        self.name
                    );
                    sleep(Duration::from_millis(1000)).await;
                    continue;
                };

                let mut post_messages = vec![];
                let mut total_bytes: u32 = 0;

                let tuples = responses.into_iter().map(|km| (km.board, km.messages));
                let km_map: HashMap<String, Vec<GrpcB3Message>> = HashMap::from_iter(tuples);

                for (k, s) in session_map.iter_mut() {
                    let empty = vec![];
                    let messages = km_map.get(k).unwrap_or(&empty);

                    // We do not want to execute the trustee step when messages is pending, this
                    // avoids executing superfluous work
                    if truncated {
                        warn!("Received truncated messages, updating only..");
                        if let Err(err) = s.update_store(messages) {
                            error!(
                                "Error updating store: {} (returned messages truncated)",
                                err
                            );
                        }
                        continue;
                    }

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

                if post_messages.len() > 0 {
                    info!(
                        "Set {}: posting {} keyed messages with {:.3} MB",
                        self.name,
                        post_messages.len(),
                        f64::from(total_bytes) / (1024.0 * 1024.0)
                    );
                    let now = std::time::Instant::now();
                    let result = board.insert_messages_multi(post_messages).await;
                    if let Err(err) = result {
                        error!("Error posting messages: '{:?} (set {})'", err, self.name);
                    }
                    info!(
                        "Set {}: messages posted in {}ms",
                        self.name,
                        now.elapsed().as_millis()
                    );
                } else {
                    // No messages to send
                }
            }
            // We should never get here
        });

        handler
    }
}

#[derive(Clone)]
pub struct SessionFactory {
    pub(crate) trustee_name: String,
    signing_key: StrandSignatureSk,
    symm_key: SymmetricKey,
    store_root: PathBuf,
    no_cache: bool,
}
impl SessionFactory {
    pub fn new(
        trustee_name: &str,
        cfg: TrusteeConfig,
        store_root: PathBuf,
        no_cache: bool,
    ) -> Result<Self> {
        let signing_key: StrandSignatureSk =
            StrandSignatureSk::from_der_b64_string(&cfg.signing_key_sk)?;

        let bytes = crate::util::decode_base64(&cfg.encryption_key)?;
        let symm_key = strand::symm::sk_from_bytes(&bytes)?;

        if !store_root.is_dir() {
            return Err(anyhow!("Invalid store root {:?}", store_root));
        }

        Ok(SessionFactory {
            trustee_name: trustee_name.to_string(),
            symm_key,
            signing_key,
            store_root,
            no_cache,
        })
    }

    pub fn create_session(&self, board_name: &str) -> Result<SessionM<RistrettoCtx>> {
        info!("* Creating new session for board '{}'..", board_name);

        let trustee: Trustee<RistrettoCtx> = Trustee::new(
            self.trustee_name.clone(),
            self.signing_key.clone(),
            self.symm_key,
            Some(self.store_root.join(&board_name)),
            self.no_cache,
        );

        SessionM::new(board_name, trustee)
    }
}
