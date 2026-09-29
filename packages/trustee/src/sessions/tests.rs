// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Session sets over in-memory boards, with real braid trustees registered from
//! their keys files, a real Configuration and real `Ballots` messages built by
//! the platform's own code, and the SQLite stores the trustee runs with.
//!
//! braid's own suite proves the protocol: equivocation halts, the anti-rewrite
//! gate, the mailbox, the mix and the decryption. These tests cover what the
//! trustee adds around it: which boards get a session, what a tally board's
//! session is read with and checked against, which errors halt a session and
//! which are retried, how long a session and a halt live, what a halt leaves to
//! report, and that a session's store is the one found again after a restart.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{bail, Result};
use async_trait::async_trait;
use cryptography::cryptosystem::{elgamal, naoryung};
use cryptography::groups::Ristretto255Group;
use cryptography::utils::serialization::{Deserializable, Serializable};
use protocol_board::{
    generate_manager, BallotCiphertext, BoardManager, BoardName, Committee,
    Ctx, DkgBoard, ElementPayload, HashHex, ProtocolBoardKind, Quorum,
    RawTrusteeRecord, TallyBoard, TrusteeBoard, TrusteeReportKind,
    TrusteeSecrets,
};
use tempfile::TempDir;
use uuid::Uuid;
use wbraid::board::persistence::NoOpPersistence;
use wbraid::board::transport::{
    MemoryBoard, MemoryTransport, StagedRef, Transport,
};
use wbraid::board::BoardClient;
use wbraid::messages::artifact::{DkgPublicKey, Plaintexts};
use wbraid::messages::newtypes::hash_bytes;
use wbraid::messages::wire::{MessageType, ProtocolMessage};
use wbraid::native::persistence::SqlitePersistence;
use wbraid::session::Session;
use wbraid::trustee::{ballot_encryption_context, Trustee};

use super::{BoardAccess, SessionSet, SessionStores};

const THRESHOLD: usize = 2;

/// Rounds after which trustees that are still posting are given up on.
const MAX_ROUNDS: usize = 20;

/// The tally session and the board row of the tally boards made here.
const TALLY_SESSION: u128 = 100;
const TALLY_ROW: u128 = 101;

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

/// What the platform keeps of a DKG board it created.
struct PlatformDkg {
    board: DkgBoard,
    manager: BoardManager,
    /// The committee's names, in Configuration order.
    committee: Vec<String>,
}

struct Harness {
    _directory: TempDir,
    boards: Boards,
    dkgs: RefCell<BTreeMap<BoardName, PlatformDkg>>,
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
            dkgs: RefCell::default(),
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
        let manager = generate_manager();
        let dkg = DkgBoard::new(
            &Uuid::from_u128(id),
            &manager,
            &Committee::new(&records).unwrap(),
            THRESHOLD,
        )
        .unwrap();
        let name = dkg.name.clone();
        self.register(&name, &dkg.configuration.to_bytes());
        let committee = records.into_iter().map(|record| record.name).collect();
        self.dkgs.borrow_mut().insert(
            name.clone(),
            PlatformDkg {
                board: dkg,
                manager,
                committee,
            },
        );
        name
    }

    /// A new tally board over a DKG board whose key has been generated,
    /// serving the platform's `Ballots` message: `payloads` encrypted under
    /// the joint key as a voter's client encrypts them (PROTOCOL.md §5.2), for
    /// `quorum` to mix and decrypt, in this order.
    fn tally_board(
        &self,
        parent: &BoardName,
        quorum: &[usize],
        payloads: &[ElementPayload],
    ) -> BoardName {
        let dkgs = self.dkgs.borrow();
        let dkg = &dkgs[parent];
        let served = self.memory(parent).snapshot();
        let public_key = served
            .iter()
            .find(|message| message.message_type == MessageType::PublicKey)
            .and_then(|message| message.body.as_ref())
            .unwrap();
        let joint_key = DkgPublicKey::<Ctx>::deser(public_key).unwrap().pk;
        let configuration = configuration_of(&self.memory(parent))
            .verify_configuration()
            .unwrap();
        let context =
            ballot_encryption_context::<Ctx>(configuration.id, &joint_key);
        let key = naoryung::PublicKey::augment(
            &elgamal::PublicKey::new(joint_key),
            &context,
        )
        .unwrap();
        let ballots = payloads
            .iter()
            .map(|payload| {
                let element =
                    Ristretto255Group::encode_30_bytes(payload).unwrap();
                BallotCiphertext::new(
                    vec!["contest".to_string()],
                    key.encrypt(&[element], &context).unwrap(),
                )
            })
            .collect();
        let chosen = quorum
            .iter()
            .map(|&member| self.members[member].name.clone())
            .collect::<Vec<_>>();
        let tally = TallyBoard::new(
            &Uuid::from_u128(TALLY_ROW),
            &Uuid::from_u128(TALLY_SESSION),
            1,
            &dkg.board,
            &recorded_hash(public_key),
            &Quorum::of_names(&dkg.board, &dkg.committee, &chosen).unwrap(),
            &dkg.manager,
            ballots,
        )
        .unwrap();
        self.register(&tally.name, &tally.input.to_bytes());
        tally.name
    }

    /// A new board serving one stored message.
    fn register(&self, name: &BoardName, stored: &[u8]) {
        let board = MemoryBoard::<Ctx>::new();
        board.push(ProtocolMessage::<Ctx>::deser(stored).unwrap());
        self.boards.borrow_mut().insert(name.clone(), board);
    }

    fn memory(&self, board: &BoardName) -> Arc<MemoryBoard<Ctx>> {
        Arc::clone(&self.boards.borrow()[board])
    }

    /// How many messages the board service holds, over every board.
    fn posted(&self) -> usize {
        self.boards
            .borrow()
            .values()
            .map(|board| board.snapshot().len())
            .sum()
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

    /// What the `Plaintexts` a member posted on a tally board decode to, in
    /// order.
    fn decrypted(
        &self,
        board: &BoardName,
        member: usize,
    ) -> Vec<ElementPayload> {
        let sent = self.sent(board, member, MessageType::Plaintexts);
        assert_eq!(sent.len(), 1, "{}", self.members[member].name);
        let body = sent[0].body.as_ref().unwrap();
        let mut payloads = Plaintexts::<Ctx, 1>::deser(body)
            .unwrap()
            .0
            .iter()
            .map(|[element]| {
                Ristretto255Group::decode_30_bytes(element).unwrap()
            })
            .collect::<Vec<_>>();
        payloads.sort();
        payloads
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

    /// The board service serves the board with the member's dealing replaced
    /// by another one: a rewrite of what it served before.
    async fn replace_dealing(&self, board: &BoardName, member: usize) {
        let replacement = self.another_dealing(board, member).await;
        let rewritten = MemoryBoard::<Ctx>::new();
        for message in self.memory(board).snapshot() {
            let replaced = message.message_type == MessageType::Shares
                && message.sender.name == self.members[member].name;
            rewritten.push(if replaced {
                replacement.clone()
            } else {
                message
            });
        }
        self.boards.borrow_mut().insert(board.clone(), rewritten);
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

fn tally(board: &BoardName, parent: &BoardName) -> TrusteeBoard {
    TrusteeBoard {
        name: board.clone(),
        kind: ProtocolBoardKind::TALLY,
        parent: Some(parent.clone()),
    }
}

/// A public key's hash as the keys ceremony records it.
fn recorded_hash(public_key: &[u8]) -> HashHex {
    let hex = hash_bytes(public_key)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    HashHex::parse(&hex).unwrap()
}

/// A payload starting with these bytes, zero-padded.
fn payload(bytes: &[u8]) -> ElementPayload {
    let mut payload = [0; 30];
    payload[..bytes.len()].copy_from_slice(bytes);
    payload
}

/// One cycle of every set, in order, after a list that answered: what the
/// daemon runs once its reports are sent.
async fn round(sets: &mut [SessionSet<TestBoards>], list: &[TrusteeBoard]) {
    for set in sets.iter_mut() {
        set.reconcile(list).await;
        set.advance().await;
    }
}

/// Rounds until one in which no trustee posted anything: every session has
/// done all it could.
async fn rounds_until_quiet(
    harness: &Harness,
    sets: &mut [SessionSet<TestBoards>],
    list: &[TrusteeBoard],
) {
    for _ in 0..MAX_ROUNDS {
        let posted = harness.posted();
        round(sets, list).await;
        if harness.posted() == posted {
            return;
        }
    }
    panic!("the trustees were still posting after {MAX_ROUNDS} rounds");
}

/// Rounds over a DKG board until its key is generated.
async fn generate_key(
    harness: &Harness,
    sets: &mut [SessionSet<TestBoards>],
    board: &BoardName,
) {
    rounds_until_quiet(harness, sets, &[dkg(board)]).await;
    harness.assert_key_generated(board);
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

/// The set's only halt is on `board`, where braid found a message it had
/// committed to no longer served.
fn assert_halted_on_a_rewrite(set: &SessionSet<TestBoards>, board: &BoardName) {
    let reports = set.pending_reports();
    assert_eq!(halted(set), vec![board.clone()]);
    let detail = &reports[0].detail;
    assert!(detail.contains("anti-rewrite violation"), "{detail}");
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

    harness.replace_dealing(&board, 0).await;

    for _ in 0..2 {
        let mut restarted = harness.start(2);
        restarted.reconcile(&list).await;
        restarted.advance().await;

        assert_halted_on_a_rewrite(&restarted, &board);
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

/// Beyond braid: a listed tally board gets a session over the board and its
/// parent, seeded from the trustee's own store of the parent's key generation:
/// the quorum mixes and decrypts the platform's ballots back to the payloads
/// they were encrypted from, and nothing is written to the parent.
#[tokio::test]
async fn a_tally_board_is_decrypted_in_sessions_over_it_and_its_parent() {
    let harness = Harness::new(3);
    let parent = harness.board(1, &[0, 1, 2]);
    let mut sets = harness.start_all();
    generate_key(&harness, &mut sets, &parent).await;

    let mut payloads = vec![
        payload(b"testing encryption"),
        payload(&[0xde, 0xad, 0xbe, 0xef]),
        payload(&[0xca, 0xfe, 0xba, 0xbe]),
    ];
    let board = harness.tally_board(&parent, &[2, 0], &payloads);
    rounds_until_quiet(&harness, &mut sets, &[tally(&board, &parent)]).await;

    payloads.sort();
    for member in [2, 0] {
        assert_eq!(harness.decrypted(&board, member), payloads);
    }
    harness.assert_key_generated(&parent);
    assert_nothing_halted(&sets);
}

/// Beyond braid: a tally board is never run over a parent the trustee has no
/// record of: a member that lost its store of the parent's key generation, and
/// a listing that names no parent, halt the board's session before anything
/// is posted, and have the halt to report.
#[tokio::test]
async fn a_tally_board_whose_parent_the_trustee_has_no_record_of_halts() {
    let harness = Harness::new(3);
    let parent = harness.board(1, &[0, 1, 2]);
    let mut sets = harness.start_all();
    generate_key(&harness, &mut sets, &parent).await;
    drop(sets);
    let board = harness.tally_board(&parent, &[0, 1], &[payload(b"lost")]);

    fs::remove_file(harness.members[0].store_file(&parent)).unwrap();
    let mut forgetful = harness.start(0);
    forgetful.reconcile(&[tally(&board, &parent)]).await;
    forgetful.advance().await;
    let reports = forgetful.pending_reports();
    assert_eq!(halted(&forgetful), vec![board.clone()]);
    let detail = &reports[0].detail;
    assert!(detail.contains(parent.as_str()), "{detail}");

    let orphan = TrusteeBoard {
        name: board.clone(),
        kind: ProtocolBoardKind::TALLY,
        parent: None,
    };
    let mut orphaned = harness.start(1);
    orphaned.reconcile(&[orphan]).await;
    orphaned.advance().await;
    assert_eq!(halted(&orphaned), vec![board.clone()]);

    assert_eq!(harness.memory(&board).snapshot().len(), 1);
}

/// Beyond braid: a tally board's parent is checked against the trustee's own
/// store of its key generation, not against what the board service serves
/// now: a parent that replaced a dealing halts every session over the tally
/// board before anything is mixed, and a restarted trustee halts again.
#[tokio::test]
async fn a_rewritten_parent_halts_the_tally_board_also_after_a_restart() {
    let harness = Harness::new(3);
    let parent = harness.board(1, &[0, 1, 2]);
    let mut sets = harness.start_all();
    generate_key(&harness, &mut sets, &parent).await;
    let board = harness.tally_board(&parent, &[0, 1], &[payload(b"rewrite")]);
    harness.replace_dealing(&parent, 0).await;
    let list = [tally(&board, &parent)];

    round(&mut sets, &list).await;
    for set in &sets {
        assert_halted_on_a_rewrite(set, &board);
    }
    drop(sets);

    let mut restarted = harness.start(0);
    restarted.reconcile(&list).await;
    restarted.advance().await;
    assert_halted_on_a_rewrite(&restarted, &board);
    assert_eq!(harness.memory(&board).snapshot().len(), 1);
}

/// Beyond braid: a tally board the list no longer names has its session
/// closed and keeps its store and its parent's; listed again, its session is
/// resumed from them and the tally completes.
#[tokio::test]
async fn an_unlisted_tally_board_keeps_both_stores_and_is_resumed_from_them() {
    let harness = Harness::new(3);
    let parent = harness.board(1, &[0, 1, 2]);
    let mut sets = harness.start_all();
    generate_key(&harness, &mut sets, &parent).await;
    let payloads = [payload(b"resumed")];
    let board = harness.tally_board(&parent, &[0, 1], &payloads);
    let list = [tally(&board, &parent)];
    round(&mut sets, &list).await;

    round(&mut sets[..1], &[]).await;
    assert!(kept(&sets[0]).is_empty());
    for store in [&board, &parent] {
        assert!(harness.members[0].store_file(store).exists());
    }

    rounds_until_quiet(&harness, &mut sets, &list).await;
    assert_eq!(harness.decrypted(&board, 0), payloads);
    assert_nothing_halted(&sets);
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
