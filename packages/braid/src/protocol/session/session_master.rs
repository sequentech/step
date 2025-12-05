// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hasher;
use tracing::{error, info, warn};

use rustc_hash::FxHasher;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

use b3::grpc::GrpcB3Message;
use strand::backend::ristretto::RistrettoCtx;

use crate::protocol::board::grpc_m::GrpcB3BoardParams;
use crate::protocol::board::{BoardFactoryMulti, BoardMulti};
use crate::protocol::session::session_m::{SessionFactory, SessionM};

// How often the session map (with trustee's LocalBoard) is cleared
// This will cause al messages in the LocalBoard to be reloaded from the
// message store
const SESSION_RESET_PERIOD: i64 = 20 * 60;

/// A collection of SessionSets that will handle active boards.
///
/// The master will create the requested number of session sets
/// on construction. It will then route updates to the set
/// of active boards to the necessary SessionSets using mspc
/// channels.
pub struct SessionMaster {
    session_sets: Vec<SessionSetHandle>,
    b3_url: String,
    session_factory: SessionFactory,
}
impl SessionMaster {
    /// Constructs a SessionMaster.
    ///
    /// On construction, a SessionMaster will construct and then
    /// start the requested number of SessionSets and the channels
    /// used to update them.
    ///
    pub fn new(b3_url: &str, session_factory: SessionFactory, size: usize) -> Result<Self> {
        let mut session_sets = vec![];
        let mut runners = vec![];
        for i in 0..size {
            let (s, r): (Sender<SessionSetMessage>, Receiver<SessionSetMessage>) =
                tokio::sync::mpsc::channel(1);
            let session_set = SessionSet::new(&i.to_string(), &session_factory, &b3_url, r)?;
            runners.push(session_set);

            let handle = SessionSetHandle::new(s);
            session_sets.push(handle);
        }

        info!("* Starting {} session sets..", runners.len());
        runners.into_iter().for_each(|r| {
            r.run();
        });

        Ok(SessionMaster {
            b3_url: b3_url.to_string(),
            session_factory,
            session_sets,
        })
    }

    /// Returns the session set that will handle this board
    fn modulo_hash(&self, board: &str) -> usize {
        let mut hasher = FxHasher::default();
        hasher.write(board.as_bytes());
        let ret = hasher.finish() % self.session_sets.len() as u64;

        ret as usize
    }

    /// Updates the active boards for children SessionSets.
    ///
    /// Active boards are assigned to SessionSets using modulo
    /// hashing. The SessionSets are notified of which boards
    /// they will handle.
    ///
    /// These notifications are idempotent, the SessionSet will
    /// add or remove active boards as necessary.
    pub async fn refresh_sets(&mut self, boards: Vec<String>) -> Result<()> {
        // info!("Refreshing {} sets with {} boards", self.session_sets.len(), boards.len());

        for board in boards {
            let index = self.modulo_hash(&board);
            // Assign boards to session sets
            self.session_sets[index].boards.push(board);
        }

        for (i, h) in self.session_sets.iter_mut().enumerate() {
            let boards = std::mem::replace(&mut h.boards, vec![]);

            if h.sender.is_closed() {
                warn!("Sender was closed, rebuilding set..");
                let (s, r): (Sender<SessionSetMessage>, Receiver<SessionSetMessage>) =
                    tokio::sync::mpsc::channel(1);
                let session_set = SessionSet::new(
                    &format!("rebuilt {}", i),
                    &self.session_factory,
                    &self.b3_url,
                    r,
                )?;
                h.sender = s;

                session_set.run();
            }
            // The only error we care about is checked above with sender.is_closed
            let _ = h.sender.try_send(SessionSetMessage::REFRESH(boards));
        }

        Ok(())
    }
}

/// A collection of SessionMs that will each handle one board.
///
/// A SessionSet will be assigned a number of active boards to
/// run protocol sessions on. SessionSets are run in tokio threads
/// concurrently. Session's individual bulletin board requests and
/// responses are multiplexed and chunked at the SessionSet level.
pub struct SessionSet {
    name: String,
    session_factory: SessionFactory,
    b3_url: String,
    inbox: Receiver<SessionSetMessage>,
}
impl SessionSet {
    /// Constructs a SessionSet, but does not run any Sessions yet.
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

    /// Spawns the SessionSet tokio thread.
    ///
    /// The SessionSet implements the main protocol loop for several boards,
    /// each handled by its own SessionM.
    ///
    /// The main steps are:
    ///
    /// 1) Gather all bulletin board requests from its SessionM's.
    /// 2) Make the requests from the bulletin board, with chunking.
    /// 3) Receive the responses from the server.
    /// 4) Distribute the messages to their handling SessionM's
    /// and run one trustee step.
    /// 5) Gather all messages produced by the SessionM's
    /// 6) Post the messages to the bulletin board.
    ///
    /// SessionSets are are notified of new or archived
    /// boards by the master using mpsc channels. A SessionSet will
    /// adjust its SessionM's to match the list of active boards.
    ///
    /// SessionSet threads run during the duration of the braid
    /// process and are not expected to exit.
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
                let batches = 10;
                let dkg_messages = 1 + 5 * trustees;
                let tally_messages = batches * (trustees + (threshold * (threshold + 1)) + 1);

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

                // Chunking: if the bulletin board returns truncated = true it means there
                // are more messages pending that were cut off to not exceed configured message
                // size limit
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

                    // Chunking: we do not want to execute the trustee step when messages are pending; this
                    // avoids executing superfluous work. This also improves core utilization.
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

                    // NOTE: We must call step even if there are no new remote messages
                    // because there may be messages in the message_store whose required
                    // Actions have not yet executed, leading to a possible protocol hang.
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
                        "Set {}: posting messages for {} boards with {:.3} MB",
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

/// An mpsc channel handle to a SessionSet.
///
/// These handles are used to update each SessionSet
/// with their set of active boards.
struct SessionSetHandle {
    boards: Vec<String>,
    sender: Sender<SessionSetMessage>,
}
impl SessionSetHandle {
    fn new(sender: Sender<SessionSetMessage>) -> Self {
        SessionSetHandle {
            boards: vec![],
            sender,
        }
    }
}

/// Specifies which boards should be active.
///
/// Used by the master to update its SessionSets
/// on what boards must be run.
pub enum SessionSetMessage {
    REFRESH(Vec<String>),
}
