// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The trustee's sessions: one braid session per board it runs, kept in step
//! with the platform's list of its boards.
//!
//! The list is a hint: a board's Configuration is the authority on whether
//! the trustee takes part in it. A session whose board service fails is tried
//! again on the next cycle; one that meets a verdict is halted for good in
//! this process, and its halt is reported until the platform has recorded it.
//! Each board's session keeps a store, which the trustee never deletes: braid
//! checks the board against it after a restart, and the store of a DKG board
//! outlives its ceremony, for the tallies over its key.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context as _, Result};
use protocol_board::{
    BoardName, Ctx, ProtocolBoardKind, TrusteeBoard, TrusteeReport,
    TrusteeReportKind, TrusteeSecrets,
};
use tracing::{error, info, warn};
use wbraid::board::persistence::Persistence;
use wbraid::board::transport::Transport;
use wbraid::board::BoardClient;
use wbraid::native::http_transport::HttpTransport;
use wbraid::native::persistence::SqlitePersistence;
use wbraid::session::Session;
use wbraid::trustee::Trustee;

use crate::transport::{classify, OutageTagged, Verdict};

#[cfg(test)]
mod tests;

/// Where a session set gets a board's transport and its session's store.
pub(crate) trait BoardAccess {
    type Transport: Transport<Ctx>;
    type Persistence: Persistence;

    /// The transport to a board. The session set tags its errors.
    fn transport(&self, board: &BoardName) -> Self::Transport;

    /// The store of a board's session, created if it does not exist.
    fn store(&self, board: &BoardName) -> Result<Self::Persistence>;
}

/// The board service, over braid's HTTP transport.
pub(crate) struct B4Boards {
    b4_url: String,
    stores: SessionStores,
}

impl B4Boards {
    pub(crate) fn new(b4_url: String, stores: SessionStores) -> B4Boards {
        B4Boards { b4_url, stores }
    }
}

impl BoardAccess for B4Boards {
    type Transport = HttpTransport;
    type Persistence = SqlitePersistence;

    fn transport(&self, board: &BoardName) -> HttpTransport {
        HttpTransport::new(&self.b4_url, board.as_str())
    }

    fn store(&self, board: &BoardName) -> Result<SqlitePersistence> {
        self.stores.open(board)
    }
}

/// The directory of the sessions' stores: one SQLite file per board, named
/// after it.
pub(crate) struct SessionStores {
    directory: PathBuf,
}

impl SessionStores {
    /// The stores under `directory`, which is created if it does not exist.
    pub(crate) fn new(directory: PathBuf) -> Result<SessionStores> {
        fs::create_dir_all(&directory).with_context(|| {
            format!(
                "creating the session store directory {}",
                directory.display()
            )
        })?;
        Ok(SessionStores { directory })
    }

    fn open(&self, board: &BoardName) -> Result<SqlitePersistence> {
        let path = self.directory.join(format!("{board}.sqlite"));
        // SQLite's error names the file, and this error may be reported to
        // the platform, which is never told a path: the file goes to the log.
        SqlitePersistence::open(&path).map_err(|err| {
            error!(
                board = %board,
                "opening the session store {} failed: {err:#}",
                path.display()
            );
            anyhow!("the session store of board {board} cannot be opened")
        })
    }
}

type BoardSession<A> = Session<
    Ctx,
    OutageTagged<<A as BoardAccess>::Transport>,
    <A as BoardAccess>::Persistence,
>;

/// What the trustee knows about one board.
enum BoardEntry<A: BoardAccess> {
    /// The board's session, with the outage its last cycle met.
    Open {
        session: Box<BoardSession<A>>,
        outage: Option<String>,
    },
    /// Opening the session met an outage: tried again on the next cycle.
    Unreachable { outage: String },
    /// Not this trustee's to run, for as long as it is listed.
    Skipped,
    /// The session halted: never advanced again in this process, and
    /// reported, listed or not, until the platform has recorded it.
    Halted { detail: String },
}

/// One trustee's sessions, one entry per board name.
pub(crate) struct SessionSet<A: BoardAccess> {
    /// The trustee's name, which braid stamps into the messages it signs.
    name: String,
    secrets: TrusteeSecrets,
    access: A,
    entries: BTreeMap<BoardName, BoardEntry<A>>,
    /// The boards of the last list, in its order.
    listed: Vec<BoardName>,
}

impl<A: BoardAccess> SessionSet<A> {
    pub(crate) fn new(
        name: String,
        secrets: TrusteeSecrets,
        access: A,
    ) -> SessionSet<A> {
        SessionSet {
            name,
            secrets,
            access,
            entries: BTreeMap::new(),
            listed: Vec::new(),
        }
    }

    /// Follow a list the platform answered: open the listed boards that have
    /// no session yet, and close the sessions of the boards it no longer
    /// lists, whose ceremonies the platform has ended. Halted entries ignore
    /// the list.
    pub(crate) async fn reconcile(&mut self, list: &[TrusteeBoard]) {
        self.entries.retain(|name, entry| {
            let listed = list.iter().any(|board| board.name == *name);
            match entry {
                BoardEntry::Open { .. } => {
                    if !listed {
                        info!(
                            board = %name,
                            "no longer listed: session closed, store kept"
                        );
                    }
                    listed
                }
                BoardEntry::Unreachable { .. } | BoardEntry::Skipped => listed,
                BoardEntry::Halted { .. } => true,
            }
        });
        for board in list {
            let last_outage = match self.entries.get(&board.name) {
                None => None,
                Some(BoardEntry::Unreachable { outage }) => {
                    Some(outage.clone())
                }
                Some(
                    BoardEntry::Open { .. }
                    | BoardEntry::Skipped
                    | BoardEntry::Halted { .. },
                ) => continue,
            };
            let entry = self.open(board, last_outage.as_deref()).await;
            self.entries.insert(board.name.clone(), entry);
        }
        self.listed = list.iter().map(|board| board.name.clone()).collect();
    }

    /// One update, step and post of every open session, in list order.
    pub(crate) async fn advance(&mut self) {
        for name in &self.listed {
            let Some(entry) = self.entries.get_mut(name) else {
                continue;
            };
            let (session, outage) = match entry {
                BoardEntry::Open { session, outage } => (session, outage),
                BoardEntry::Unreachable { .. }
                | BoardEntry::Skipped
                | BoardEntry::Halted { .. } => continue,
            };
            match cycle(&mut **session).await {
                Ok(()) => {
                    if outage.take().is_some() {
                        info!(board = %name, "the board service answers again");
                    }
                }
                Err(err) => match classify(&err) {
                    Verdict::Outage => {
                        let text = format!("{err:#}");
                        if outage.as_deref() != Some(text.as_str()) {
                            warn!(board = %name, "{text}");
                        }
                        *outage = Some(text);
                    }
                    Verdict::Halt => {
                        let detail = format!("{err:#}");
                        error!(board = %name, "session halted: {detail}");
                        *entry = BoardEntry::Halted { detail };
                    }
                },
            }
        }
    }

    /// A report for every halt the platform has not recorded yet.
    pub(crate) fn pending_reports(&self) -> Vec<TrusteeReport> {
        self.entries
            .iter()
            .filter_map(|(board, entry)| match entry {
                BoardEntry::Halted { detail } => Some(TrusteeReport {
                    board: board.clone(),
                    kind: TrusteeReportKind::HALTED,
                    detail: detail.clone(),
                }),
                BoardEntry::Open { .. }
                | BoardEntry::Unreachable { .. }
                | BoardEntry::Skipped => None,
            })
            .collect()
    }

    /// The platform has recorded the halt on `board`.
    pub(crate) fn settle(&mut self, board: &BoardName) {
        if let Some(BoardEntry::Halted { .. }) = self.entries.get(board) {
            self.entries.remove(board);
        }
    }

    async fn open(
        &self,
        board: &TrusteeBoard,
        last_outage: Option<&str>,
    ) -> BoardEntry<A> {
        match board.kind {
            ProtocolBoardKind::DKG => {
                self.open_dkg(&board.name, last_outage).await
            }
            ProtocolBoardKind::TALLY => {
                warn!(
                    board = %board.name,
                    parent = ?board.parent.as_ref().map(BoardName::as_str),
                    "tally boards are not run yet: skipped while listed"
                );
                BoardEntry::Skipped
            }
        }
    }

    async fn open_dkg(
        &self,
        name: &BoardName,
        last_outage: Option<&str>,
    ) -> BoardEntry<A> {
        match self.connect(name).await {
            Ok(Some(session)) => {
                info!(board = %name, "session opened");
                BoardEntry::Open {
                    session: Box::new(session),
                    outage: None,
                }
            }
            Ok(None) => {
                error!(
                    board = %name,
                    "the board's Configuration does not name this trustee's \
                     signing key: skipped while listed"
                );
                BoardEntry::Skipped
            }
            Err(err) => match classify(&err) {
                Verdict::Outage => {
                    let outage = format!("{err:#}");
                    if last_outage != Some(outage.as_str()) {
                        warn!(
                            board = %name,
                            "cannot open the session: {outage}"
                        );
                    }
                    BoardEntry::Unreachable { outage }
                }
                Verdict::Halt => {
                    let detail = format!("{err:#}");
                    error!(board = %name, "session halted: {detail}");
                    BoardEntry::Halted { detail }
                }
            },
        }
    }

    /// The session over a board, or `None` when the board's Configuration does
    /// not name this trustee.
    async fn connect(
        &self,
        name: &BoardName,
    ) -> Result<Option<BoardSession<A>>> {
        let store = self.access.store(name)?;
        let transport = OutageTagged::new(self.access.transport(name));
        let client = BoardClient::<Ctx, _, _>::connect(transport, store)
            .await
            .context("connecting to the board")?;
        if !self.secrets.is_listed_in(client.configuration()) {
            return Ok(None);
        }
        let (signing_key, share_encryption) = self.secrets.clone().into_parts();
        let trustee = Trustee::new(
            self.name.clone(),
            signing_key,
            share_encryption,
            client.configuration(),
        )
        .context("taking part in the board's Configuration")?;
        Ok(Some(Session::new(trustee, client)))
    }
}

/// One update-first cycle of a session, phase by phase.
async fn cycle<T: Transport<Ctx>, P: Persistence>(
    session: &mut Session<Ctx, T, P>,
) -> Result<()> {
    session.update().await.context("updating from the board")?;
    let messages = session.step().context("running the protocol")?;
    session.post(messages).await.context("posting to the board")
}
