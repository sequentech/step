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
//! outlives its ceremony, for the tallies over its key. A tally board's session
//! reads the board together with its parent DKG board, which braid checks
//! against what this trustee's own session over the parent committed to, as
//! that session's store kept it.

use std::collections::BTreeMap;
#[cfg(feature = "native")]
use std::fs;
#[cfg(feature = "native")]
use std::path::PathBuf;
#[cfg(feature = "native")]
use std::sync::Arc;

use crate::{
    BoardName, Ctx, ProtocolBoardKind, TrusteeBoard, TrusteeReport,
    TrusteeReportKind, TrusteeSecrets,
};
use anyhow::{anyhow, bail, ensure, Context as _, Result};
use tracing::{error, info, warn};
use uuid::Uuid;
use wbraid::board::persistence::Persistence;
/// What an embedder implements to add headers, such as a platform access
/// token, to the requests to the board service.
pub use wbraid::board::transport::RequestHeaders;
use wbraid::board::transport::Transport;
use wbraid::board::BoardClient;
use wbraid::messages::newtypes::ConfigurationHash;
use wbraid::messages::predicate::Predicate;
#[cfg(feature = "native")]
use wbraid::native::http_transport::HttpTransport;
#[cfg(feature = "native")]
use wbraid::native::persistence::SqlitePersistence;
use wbraid::session::Session;
use wbraid::trustee::Trustee;

use crate::trustee_transport::{classify, OutageTagged, Verdict};

#[cfg(all(test, feature = "native"))]
mod tests;

/// Where a session set gets a board's transport and its session's store.
#[async_trait::async_trait(?Send)]
pub trait BoardAccess {
    type Transport: Transport<Ctx>;
    type Persistence: Persistence;

    /// The transport to a board. The session set tags its errors.
    fn transport(&self, board: &BoardName) -> Self::Transport;

    /// The store of a board's session, created if it does not exist.
    async fn store(&self, board: &BoardName) -> Result<Self::Persistence>;
}

/// The board service, over braid's HTTP transport.
#[cfg(feature = "native")]
pub struct B4Boards {
    b4_url: String,
    stores: SessionStores,
    /// Headers for every request to the board service; none when absent.
    headers: Option<Arc<dyn RequestHeaders + Send + Sync>>,
}

#[cfg(feature = "native")]
impl B4Boards {
    pub fn new(b4_url: String, stores: SessionStores) -> B4Boards {
        B4Boards {
            b4_url,
            stores,
            headers: None,
        }
    }

    /// Send `headers` with every request to the board service, as a gateway
    /// in front of it requires. Presigned storage URLs never get them.
    pub fn with_headers(
        self,
        headers: Arc<dyn RequestHeaders + Send + Sync>,
    ) -> B4Boards {
        B4Boards {
            headers: Some(headers),
            ..self
        }
    }
}

#[cfg(feature = "native")]
#[async_trait::async_trait(?Send)]
impl BoardAccess for B4Boards {
    type Transport = HttpTransport;
    type Persistence = SqlitePersistence;

    fn transport(&self, board: &BoardName) -> HttpTransport {
        let transport = HttpTransport::new(&self.b4_url, board.as_str());
        match &self.headers {
            Some(headers) => transport.with_headers(Arc::clone(headers)),
            None => transport,
        }
    }

    async fn store(&self, board: &BoardName) -> Result<SqlitePersistence> {
        self.stores.open(board)
    }
}

/// The directory of the sessions' stores: one SQLite file per board, named
/// after it.
#[cfg(feature = "native")]
pub struct SessionStores {
    directory: PathBuf,
}

#[cfg(feature = "native")]
impl SessionStores {
    /// The stores under `directory`, which is created if it does not exist.
    pub fn new(directory: PathBuf) -> Result<SessionStores> {
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
pub struct SessionSet<A: BoardAccess> {
    /// The trustee's name, which braid stamps into the messages it signs.
    name: String,
    secrets: TrusteeSecrets,
    access: A,
    entries: BTreeMap<BoardName, BoardEntry<A>>,
    /// The boards of the last list, in its order.
    listed: Vec<BoardName>,
}

impl<A: BoardAccess> SessionSet<A> {
    pub fn new(
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
    pub async fn reconcile(&mut self, list: &[TrusteeBoard]) {
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
    pub async fn advance(&mut self) {
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
    pub fn pending_reports(&self) -> Vec<TrusteeReport> {
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
    pub fn settle(&mut self, board: &BoardName) {
        if let Some(BoardEntry::Halted { .. }) = self.entries.get(board) {
            self.entries.remove(board);
        }
    }

    async fn open(
        &self,
        board: &TrusteeBoard,
        last_outage: Option<&str>,
    ) -> BoardEntry<A> {
        let name = &board.name;
        let connected = match board.kind {
            ProtocolBoardKind::DKG => self.connect(name).await,
            ProtocolBoardKind::TALLY => {
                self.connect_union(name, board.parent.as_ref()).await
            }
        };
        match connected {
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

    /// The session over a DKG board, or `None` when the board's Configuration
    /// does not name this trustee.
    async fn connect(
        &self,
        name: &BoardName,
    ) -> Result<Option<BoardSession<A>>> {
        let store = self.access.store(name).await?;
        let transport = OutageTagged::new(self.access.transport(name));
        let client = BoardClient::<Ctx, _, _>::connect(transport, store)
            .await
            .context("connecting to the board")?;
        let configuration = client.configuration();
        if !self.secrets.is_listed_in(configuration) {
            return Ok(None);
        }
        let original = BoardName::for_dkg(&Uuid::from_u128(configuration.id));
        let client = if original != *name {
            // A remapped DKG is the same cryptographic execution. Preserve the
            // trustee's observations AND its handed-off slots before inference
            // can run. Original staged handles belong to another board; pin
            // their predicates instead, so missing archived bodies halt before
            // the trustee can regenerate an already handed-off artifact.
            let original_store = self.access.store(&original).await?;
            let mut records = original_store.load().await?;
            records.extend(
                original_store
                    .load_own_posts()
                    .await?
                    .into_iter()
                    .map(|(predicate, _)| predicate),
            );
            ensure!(
                !records.is_empty(),
                "the original DKG session store is empty: this trustee has no record of its key generation"
            );
            let expected =
                ConfigurationHash::from_configuration(configuration)?;
            ensure!(
                records
                    .iter()
                    .all(|predicate| predicate.get_configuration() == expected),
                "the original DKG record names a different configuration"
            );
            let mut store = self.access.store(name).await?;
            for predicate in records {
                store.persist(&predicate).await?;
            }
            BoardClient::<Ctx, _, _>::connect(
                OutageTagged::new(self.access.transport(name)),
                store,
            )
            .await
            .context("connecting to the imported board")?
        } else {
            client
        };
        self.join(client)
    }

    /// The session over a tally board read together with its parent DKG
    /// board, or `None` when the parent's Configuration does not name this
    /// trustee.
    async fn connect_union(
        &self,
        name: &BoardName,
        parent: Option<&BoardName>,
    ) -> Result<Option<BoardSession<A>>> {
        let parent = parent.with_context(|| {
            format!("tally board {name} is listed without its parent board")
        })?;
        let seed = self.seed(parent).await?;
        let store = self.access.store(name).await?;
        let client = BoardClient::<Ctx, _, _>::connect_union(
            OutageTagged::new(self.access.transport(name)),
            OutageTagged::new(self.access.transport(parent)),
            store,
            seed,
        )
        .await
        .with_context(|| {
            format!("connecting to the board and its parent board {parent}")
        })?;
        self.join(client)
    }

    /// What this trustee's session over a DKG board committed to, as that
    /// session's store kept it: what braid checks the board against when one
    /// of its tally boards is read together with it. An empty store means this
    /// trustee has no record of that key generation, and the board is then
    /// never taken on trust.
    async fn seed(&self, parent: &BoardName) -> Result<Vec<Predicate>> {
        let mut seed =
            self.access.store(parent).await?.load().await.with_context(
                || format!("reading the session store of board {parent}"),
            )?;
        if seed.is_empty() {
            // Import remaps the platform board name while retaining the signed
            // configuration's immutable ID. Reuse only this trustee's own
            // observations of that original DKG; connect_union still checks
            // every parent commitment against them before the tally runs.
            let message = OutageTagged::new(self.access.transport(parent))
                .fetch_configuration()
                .await
                .with_context(|| {
                    format!(
                        "reading the configuration of parent board {parent}"
                    )
                })?;
            let configuration = message
                .verify_configuration()
                .context("verifying the parent board's configuration")?;
            let original =
                BoardName::for_dkg(&Uuid::from_u128(configuration.id));
            if original != *parent {
                seed = self.access.store(&original).await?.load().await
                    .with_context(|| format!("reading the original session store of board {original}"))?;
                let expected =
                    ConfigurationHash::from_configuration(&configuration)?;
                ensure!(
                    seed.iter()
                        .all(|predicate| predicate.get_configuration()
                            == expected),
                    "the original DKG record names a different configuration"
                );
            }
        }
        if seed.is_empty() {
            bail!(
                "the session store of parent board {parent} is empty: this \
                 trustee has no record of its key generation"
            );
        }
        Ok(seed)
    }

    /// This trustee's session over a connected board, or `None` when the
    /// board's Configuration does not name this trustee.
    fn join(
        &self,
        client: BoardClient<Ctx, OutageTagged<A::Transport>, A::Persistence>,
    ) -> Result<Option<BoardSession<A>>> {
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

#[cfg(all(test, feature = "native"))]
mod header_tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use super::*;

    struct FixedHeaders;

    #[async_trait::async_trait(?Send)]
    impl RequestHeaders for FixedHeaders {
        async fn headers(&self) -> Result<Vec<(String, String)>> {
            Ok(vec![("Authorization".into(), "Bearer board-token".into())])
        }
    }

    /// A board service that answers one request with HTTP 503 and returns
    /// its request line and headers, lowercased.
    fn board_service() -> (String, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/b4/", listener.local_addr().unwrap());
        let (waiting, ready) = mpsc::sync_channel(1);
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock =>
                    {
                        // Let the caller connect only after this bounded poll
                        // has run, so scheduling cannot hide its coverage.
                        let _ = waiting.try_send(());
                        assert!(Instant::now() < deadline, "no request");
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept failed: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut head = Vec::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                head.push(line.trim_end().to_lowercase());
            }
            write!(
                stream,
                "HTTP/1.1 503 Fixture\r\nContent-Length: 0\r\n\
                 Connection: close\r\n\r\n"
            )
            .unwrap();
            head
        });
        ready
            .recv_timeout(Duration::from_secs(5))
            .expect("board fixture did not begin waiting for a connection");
        (url, server)
    }

    async fn request_head(boards: impl Fn(String) -> B4Boards) -> Vec<String> {
        let (url, server) = board_service();
        let board =
            BoardName::parse("dkg_d9792af071b849528aac94bc0fead5f7").unwrap();
        let error =
            boards(url).transport(&board).fetch_raw().await.unwrap_err();
        assert!(format!("{error:#}").contains("503"), "{error:#}");
        server.join().unwrap()
    }

    #[tokio::test]
    async fn b4_boards_send_their_headers_to_the_board_service_under_its_path()
    {
        let directory = tempfile::tempdir().unwrap();
        let stores = || SessionStores::new(directory.path().to_path_buf());
        let with = request_head(|url| {
            B4Boards::new(url, stores().unwrap())
                .with_headers(Arc::new(FixedHeaders))
        })
        .await;
        assert_eq!(
            with[0],
            "get /b4/boards/dkg_d9792af071b849528aac94bc0fead5f7/messages \
             http/1.1"
        );
        assert!(
            with.contains(&"authorization: bearer board-token".to_owned()),
            "{with:?}"
        );
        let without =
            request_head(|url| B4Boards::new(url, stores().unwrap())).await;
        assert!(
            !without
                .iter()
                .any(|line| line.starts_with("authorization:")),
            "{without:?}"
        );
    }
}
