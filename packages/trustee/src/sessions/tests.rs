// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Session sets over in-memory boards, with real braid trustees registered from
//! their keys files, a real Configuration built by the platform's own code, and
//! the SQLite stores the trustee runs with.
//!
//! braid's own suite proves the protocol: equivocation halts, the anti-rewrite
//! gate, the mailbox. These tests cover what the trustee adds around it: which
//! boards get a session, which errors halt it and which are retried, how long a
//! session and a halt live, what a halt leaves to report, and that a session's
//! store is the one found again after a restart.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{bail, Result};
use async_trait::async_trait;
use cryptography::utils::serialization::{Deserializable, Serializable};
use protocol_board::{
    generate_manager, BoardName, Committee, Ctx, DkgBoard, ProtocolBoardKind,
    RawTrusteeRecord, TrusteeBoard, TrusteeReportKind, TrusteeSecrets,
};
use tempfile::TempDir;
use uuid::Uuid;
use wbraid::board::persistence::NoOpPersistence;
use wbraid::board::transport::{
    MemoryBoard, MemoryTransport, StagedRef, Transport,
};
use wbraid::board::BoardClient;
use wbraid::messages::wire::{MessageType, ProtocolMessage};
use wbraid::native::persistence::SqlitePersistence;
use wbraid::session::Session;
use wbraid::trustee::Trustee;

use super::{BoardAccess, SessionSet, SessionStores};

const THRESHOLD: usize = 2;

/// The board service's boards by name, shared by every trustee.
type Boards = Rc<RefCell<BTreeMap<BoardName, Arc<MemoryBoard<Ctx>>>>>;

/// What the board service holds staged for one trustee.
type Staged = Rc<RefCell<Vec<ProtocolMessage<Ctx>>>>;

/// Failures to inject into one trustee's view of the board service.
#[derive(Default)]
struct Faults {
    fetch: Cell<bool>,
    commit: Cell<bool>,
}

/// The board service as braid's HTTP transport sees it: a staged message is
/// published by its commit, so a commit can fail after the body was staged.
struct TestTransport {
    board: Arc<MemoryBoard<Ctx>>,
    staged: Staged,
    faults: Rc<Faults>,
}

fn unless_down(fault: &Cell<bool>) -> Result<()> {
    if fault.get() {
        bail!("the board service is down");
    }
    Ok(())
}

#[async_trait(?Send)]
impl Transport<Ctx> for TestTransport {
    async fn fetch_configuration(&self) -> Result<ProtocolMessage<Ctx>> {
        unless_down(&self.faults.fetch)?;
        MemoryTransport::new(Arc::clone(&self.board))
            .fetch_configuration()
            .await
    }

    async fn fetch(&self) -> Result<Vec<ProtocolMessage<Ctx>>> {
        unless_down(&self.faults.fetch)?;
        MemoryTransport::new(Arc::clone(&self.board)).fetch().await
    }

    async fn stage(&self, message: &ProtocolMessage<Ctx>) -> Result<StagedRef> {
        let mut staged = self.staged.borrow_mut();
        staged.push(message.clone());
        Ok(StagedRef((staged.len() - 1).to_string()))
    }

    async fn commit(&self, staged: &StagedRef) -> Result<()> {
        unless_down(&self.faults.commit)?;
        let index: usize = staged.0.parse()?;
        self.board.push(self.staged.borrow()[index].clone());
        Ok(())
    }
}

/// One trustee's view of the board service, and its store directory.
struct TestBoards {
    boards: Boards,
    staged: Staged,
    faults: Rc<Faults>,
    stores: SessionStores,
}

impl BoardAccess for TestBoards {
    type Transport = TestTransport;
    type Persistence = SqlitePersistence;

    fn transport(&self, board: &BoardName) -> TestTransport {
        TestTransport {
            board: Arc::clone(&self.boards.borrow()[board]),
            staged: Rc::clone(&self.staged),
            faults: Rc::clone(&self.faults),
        }
    }

    fn store(&self, board: &BoardName) -> Result<SqlitePersistence> {
        self.stores.open(board)
    }
}

/// One trustee, with what outlives its process: the secrets of its keys file,
/// its store directory, and what the board service holds staged for it.
struct Member {
    name: String,
    secrets: TrusteeSecrets,
    stores: PathBuf,
    staged: Staged,
    faults: Rc<Faults>,
}

impl Member {
    /// The trustee row an administrator registers from its printed keys.
    fn record(&self) -> RawTrusteeRecord {
        let keys = self.secrets.public_keys().unwrap();
        RawTrusteeRecord {
            name: self.name.clone(),
            signing_public_key: Some(keys.signing_public_key),
            share_encryption_public_key: Some(keys.share_encryption_public_key),
        }
    }

    fn store_file(&self, board: &BoardName) -> PathBuf {
        self.stores.join(format!("{board}.sqlite"))
    }
}

struct Harness {
    _directory: TempDir,
    boards: Boards,
    members: Vec<Member>,
}

impl Harness {
    fn new(count: usize) -> Harness {
        let directory = tempfile::tempdir().unwrap();
        let members = (1..=count)
            .map(|n| Member {
                name: format!("trustee{n}"),
                secrets: TrusteeSecrets::generate(),
                stores: directory
                    .path()
                    .join(format!("trustee{n}"))
                    .join("message_store"),
                staged: Staged::default(),
                faults: Rc::default(),
            })
            .collect();
        Harness {
            _directory: directory,
            boards: Boards::default(),
            members,
        }
    }

    /// A new board serving the Configuration of a key generation among these
    /// members, in this order.
    fn board(&self, id: u128, members: &[usize]) -> BoardName {
        let records = members
            .iter()
            .map(|&member| self.members[member].record())
            .collect::<Vec<_>>();
        let dkg = DkgBoard::new(
            &Uuid::from_u128(id),
            &generate_manager(),
            &Committee::new(&records).unwrap(),
            THRESHOLD,
        )
        .unwrap();
        let board = MemoryBoard::<Ctx>::new();
        board.push(
            ProtocolMessage::<Ctx>::deser(&dkg.configuration.to_bytes())
                .unwrap(),
        );
        self.boards.borrow_mut().insert(dkg.name.clone(), board);
        dkg.name
    }

    fn memory(&self, board: &BoardName) -> Arc<MemoryBoard<Ctx>> {
        Arc::clone(&self.boards.borrow()[board])
    }

    /// The member's session set as `trustee start` builds it.
    fn start(&self, member: usize) -> SessionSet<TestBoards> {
        let member = &self.members[member];
        let access = TestBoards {
            boards: Rc::clone(&self.boards),
            staged: Rc::clone(&member.staged),
            faults: Rc::clone(&member.faults),
            stores: SessionStores::new(member.stores.clone()).unwrap(),
        };
        SessionSet::new(member.name.clone(), member.secrets.clone(), access)
    }

    fn start_all(&self) -> Vec<SessionSet<TestBoards>> {
        (0..self.members.len())
            .map(|member| self.start(member))
            .collect()
    }

    /// The messages of a type a member signed on a board.
    fn sent(
        &self,
        board: &BoardName,
        member: usize,
        kind: MessageType,
    ) -> Vec<ProtocolMessage<Ctx>> {
        let name = &self.members[member].name;
        self.memory(board)
            .snapshot()
            .into_iter()
            .filter(|message| {
                message.sender.name == *name && message.message_type == kind
            })
            .collect()
    }

    /// Every member posted one dealing and one public key, and nothing else.
    fn assert_key_generated(&self, board: &BoardName) {
        for (member, trustee) in self.members.iter().enumerate() {
            for kind in [MessageType::Shares, MessageType::PublicKey] {
                let sent = self.sent(board, member, kind);
                assert_eq!(sent.len(), 1, "{} {kind:?}", trustee.name);
            }
        }
        let messages = self.memory(board).snapshot().len();
        assert_eq!(messages, 1 + 2 * self.members.len());
    }

    /// The member's only dealing on the board is the first message it ever
    /// staged.
    fn assert_first_staged_dealing_published(
        &self,
        board: &BoardName,
        member: usize,
    ) {
        let staged = self.members[member].staged.borrow()[0].ser();
        let published = self.sent(board, member, MessageType::Shares);
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].ser(), staged);
    }

    /// Another dealing of a member for a board, as its session would have
    /// computed it with other randomness: a different message for the same
    /// slot.
    async fn another_dealing(
        &self,
        board: &BoardName,
        member: usize,
    ) -> ProtocolMessage<Ctx> {
        let copy = MemoryBoard::<Ctx>::new();
        copy.push(configuration_of(&self.memory(board)));
        let client = BoardClient::connect(
            MemoryTransport::new(Arc::clone(&copy)),
            NoOpPersistence,
        )
        .await
        .unwrap();
        let member = &self.members[member];
        let (signing_key, share_encryption) =
            member.secrets.clone().into_parts();
        let trustee = Trustee::new(
            member.name.clone(),
            signing_key,
            share_encryption,
            client.configuration(),
        )
        .unwrap();
        Session::new(trustee, client).advance().await.unwrap();
        copy.snapshot()
            .into_iter()
            .find(|message| message.message_type == MessageType::Shares)
            .unwrap()
    }
}

fn configuration_of(board: &MemoryBoard<Ctx>) -> ProtocolMessage<Ctx> {
    board
        .snapshot()
        .into_iter()
        .find(|message| message.message_type == MessageType::Configuration)
        .unwrap()
}

fn dkg(board: &BoardName) -> TrusteeBoard {
    TrusteeBoard {
        name: board.clone(),
        kind: ProtocolBoardKind::DKG,
        parent: None,
    }
}

/// One cycle of every set, in order, after a list that answered: what the
/// daemon runs once its reports are sent.
async fn round(sets: &mut [SessionSet<TestBoards>], list: &[TrusteeBoard]) {
    for set in sets.iter_mut() {
        set.reconcile(list).await;
        set.advance().await;
    }
}

/// The boards a set has a halt to report on.
fn halted(set: &SessionSet<TestBoards>) -> Vec<BoardName> {
    set.pending_reports()
        .into_iter()
        .map(|report| report.board)
        .collect()
}

/// The boards a set keeps an entry for, whatever its state.
fn kept(set: &SessionSet<TestBoards>) -> Vec<BoardName> {
    set.entries.keys().cloned().collect()
}

fn assert_nothing_halted(sets: &[SessionSet<TestBoards>]) {
    for set in sets {
        assert!(halted(set).is_empty(), "{:?}", set.pending_reports());
    }
}

/// Beyond braid: the sets open one session per listed board, give braid each
/// trustee's own name and secrets, and keep every session going from cycle to
/// cycle until the key generation is over.
#[tokio::test]
async fn three_trustees_generate_a_key_each_posting_its_own_messages_once() {
    let harness = Harness::new(3);
    let board = harness.board(1, &[0, 1, 2]);
    let list = [dkg(&board)];
    let mut sets = harness.start_all();

    for _ in 0..3 {
        round(&mut sets, &list).await;
    }

    harness.assert_key_generated(&board);
    assert_nothing_halted(&sets);
}

/// Beyond braid: braid's halt becomes a halted entry of that board only, in
/// every trustee; the entry is reported with braid's error, outlives the
/// board's listing and goes once the platform has recorded it, while the
/// trustees' sessions over another board go on.
#[tokio::test]
async fn an_equivocation_halts_its_board_only_and_is_reported_until_settled() {
    let harness = Harness::new(3);
    let tampered = harness.board(1, &[0, 1, 2]);
    let clean = harness.board(2, &[0, 1, 2]);
    let list = [dkg(&tampered), dkg(&clean)];
    let mut sets = harness.start_all();

    round(&mut sets, &list).await;
    let equivocation = harness.another_dealing(&tampered, 0).await;
    harness.memory(&tampered).push(equivocation);
    for _ in 0..3 {
        round(&mut sets, &list).await;
    }

    harness.assert_key_generated(&clean);
    for set in &sets {
        let reports = set.pending_reports();
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert_eq!(reports[0].board, tampered);
        assert_eq!(reports[0].kind, TrusteeReportKind::HALTED);
        let detail = &reports[0].detail;
        assert!(detail.contains("datalog reported errors"), "{detail}");
    }

    let set = &mut sets[1];
    set.reconcile(&[dkg(&clean)]).await;
    set.advance().await;
    assert_eq!(halted(set), vec![tampered.clone()]);
    set.settle(&tampered);
    assert!(halted(set).is_empty());
}

/// Beyond braid: a transport failure in an update is an outage, not a halt:
/// nothing is reported and the next cycle goes on where the session was.
#[tokio::test]
async fn an_outage_in_an_update_is_retried_and_never_reported() {
    let harness = Harness::new(3);
    let board = harness.board(1, &[0, 1, 2]);
    let list = [dkg(&board)];
    let mut sets = harness.start_all();
    round(&mut sets, &list).await;

    harness.members[1].faults.fetch.set(true);
    round(&mut sets, &list).await;
    assert!(harness.sent(&board, 1, MessageType::PublicKey).is_empty());
    assert_nothing_halted(&sets);

    harness.members[1].faults.fetch.set(false);
    round(&mut sets, &list).await;
    harness.assert_key_generated(&board);
    assert_nothing_halted(&sets);
}

/// Beyond braid: a failed commit is an outage, not a halt, and the store a
/// restarted trustee reopens for a board is the one its session wrote before,
/// one file per board named after it, so a post it never saw committed is
/// published as recorded.
#[tokio::test]
async fn a_restarted_trustee_publishes_the_post_its_store_recorded() {
    let harness = Harness::new(3);
    let board = harness.board(1, &[0, 1, 2]);
    let list = [dkg(&board)];
    let mut sets = harness.start_all();

    harness.members[0].faults.commit.set(true);
    round(&mut sets, &list).await;
    assert!(harness.sent(&board, 0, MessageType::Shares).is_empty());
    assert_nothing_halted(&sets);
    harness.members[0].faults.commit.set(false);
    assert!(harness.members[0].store_file(&board).exists());

    sets[0] = harness.start(0);
    for _ in 0..2 {
        round(&mut sets, &list).await;
    }
    harness.assert_key_generated(&board);
    harness.assert_first_staged_dealing_published(&board, 0);
    assert_nothing_halted(&sets);
}

/// Beyond braid: a trustee restarted over a board that replaced a dealing it
/// had served halts, which only the reloaded store can make it do; restarted
/// again, it forgets the halt, halts again and has it to report again.
#[tokio::test]
async fn a_board_rewritten_across_restarts_halts_the_trustee_after_each() {
    let harness = Harness::new(3);
    let board = harness.board(1, &[0, 1, 2]);
    let list = [dkg(&board)];
    // The third trustee sees the first two dealings before it deals.
    let mut sets = harness.start_all();
    round(&mut sets, &list).await;
    drop(sets);

    let replacement = harness.another_dealing(&board, 0).await;
    let rewritten = MemoryBoard::<Ctx>::new();
    for message in harness.memory(&board).snapshot() {
        let replaced = message.message_type == MessageType::Shares
            && message.sender.name == harness.members[0].name;
        rewritten.push(if replaced {
            replacement.clone()
        } else {
            message
        });
    }
    harness.boards.borrow_mut().insert(board.clone(), rewritten);

    for _ in 0..2 {
        let mut restarted = harness.start(2);
        restarted.reconcile(&list).await;
        restarted.advance().await;

        let reports = restarted.pending_reports();
        assert_eq!(halted(&restarted), vec![board.clone()]);
        let detail = &reports[0].detail;
        assert!(detail.contains("anti-rewrite violation"), "{detail}");
        assert!(harness.sent(&board, 2, MessageType::PublicKey).is_empty());
    }
}

/// Beyond braid: a board listed before the board service serves its
/// Configuration is an outage while opening, retried every cycle and never
/// reported, and run once the Configuration is there.
#[tokio::test]
async fn a_board_without_its_configuration_yet_is_opened_once_it_has_one() {
    let harness = Harness::new(3);
    let board = harness.board(1, &[0, 1, 2]);
    let configuration = configuration_of(&harness.memory(&board));
    harness
        .boards
        .borrow_mut()
        .insert(board.clone(), MemoryBoard::<Ctx>::new());
    let list = [dkg(&board)];
    let mut sets = harness.start_all();

    for _ in 0..2 {
        round(&mut sets, &list).await;
    }
    assert!(harness.memory(&board).snapshot().is_empty());
    assert_nothing_halted(&sets);

    harness.memory(&board).push(configuration);
    for _ in 0..2 {
        round(&mut sets, &list).await;
    }
    harness.assert_key_generated(&board);
}

/// Beyond braid: a board whose session cannot be opened for a reason other than
/// an outage gets a halted entry, reported without the store's path, and is
/// not reopened while its report is pending.
#[tokio::test]
async fn a_store_that_cannot_be_opened_halts_its_board_and_is_reported_without_its_path(
) {
    let harness = Harness::new(3);
    let board = harness.board(1, &[0, 1, 2]);
    let list = [dkg(&board)];
    let store = harness.members[0].store_file(&board);
    fs::create_dir_all(&store).unwrap();
    let mut set = harness.start(0);

    set.reconcile(&list).await;
    set.advance().await;
    let reports = set.pending_reports();
    assert_eq!(halted(&set), vec![board.clone()]);
    let detail = &reports[0].detail;
    assert!(detail.contains(board.as_str()), "{detail}");
    let stores = harness.members[0].stores.to_str().unwrap();
    assert!(!detail.contains(stores), "{detail}");
    assert_eq!(harness.memory(&board).snapshot().len(), 1);

    fs::remove_dir(&store).unwrap();
    set.reconcile(&list).await;
    set.advance().await;
    assert_eq!(halted(&set), vec![board.clone()]);
    assert_eq!(harness.memory(&board).snapshot().len(), 1);
}

/// Beyond braid: a board the list no longer names has its session closed, is
/// not worked on, keeps its store, and is resumed when it is listed again.
#[tokio::test]
async fn an_unlisted_board_is_left_alone_and_keeps_its_store() {
    let harness = Harness::new(3);
    let board = harness.board(1, &[0, 1, 2]);
    let list = [dkg(&board)];
    let mut sets = harness.start_all();
    round(&mut sets, &list).await;

    round(&mut sets[..1], &[]).await;
    assert!(kept(&sets[0]).is_empty());
    round(&mut sets[1..], &list).await;
    assert!(harness.sent(&board, 0, MessageType::PublicKey).is_empty());
    assert!(harness.members[0].store_file(&board).exists());

    round(&mut sets, &list).await;
    harness.assert_key_generated(&board);
    assert_nothing_halted(&sets);
}

/// Beyond braid: a tally board gets no session, no store and no report, even
/// over a Configuration that names the trustee.
#[tokio::test]
async fn a_tally_board_is_skipped() {
    let harness = Harness::new(3);
    let parent = harness.board(1, &[0, 1, 2]);
    let tally = harness.board(2, &[0, 1, 2]);
    let list = [TrusteeBoard {
        name: tally.clone(),
        kind: ProtocolBoardKind::TALLY,
        parent: Some(parent),
    }];
    let mut sets = harness.start_all();

    for _ in 0..2 {
        round(&mut sets, &list).await;
    }
    assert_eq!(harness.memory(&tally).snapshot().len(), 1);
    assert_nothing_halted(&sets);
    for member in &harness.members {
        assert!(!member.store_file(&tally).exists());
    }
}

/// Beyond braid: a listed board whose Configuration does not name the
/// trustee's signing key, where braid would refuse to build the trustee, is
/// skipped without posting, is not a halt, and is forgotten once unlisted.
#[tokio::test]
async fn a_board_whose_configuration_does_not_name_the_trustee_is_skipped() {
    let harness = Harness::new(3);
    let board = harness.board(1, &[1, 2]);
    let list = [dkg(&board)];
    let mut outsider = harness.start(0);

    for _ in 0..2 {
        outsider.reconcile(&list).await;
        outsider.advance().await;
    }
    assert_eq!(harness.memory(&board).snapshot().len(), 1);
    assert!(halted(&outsider).is_empty());

    outsider.reconcile(&[]).await;
    assert!(kept(&outsider).is_empty());
}
