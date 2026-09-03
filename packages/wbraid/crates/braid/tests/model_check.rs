// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Explicit-state model checking of the trustee protocol over the **real**
//! implementation (§7.6, §12 "Assurance").
//!
//! What makes this different from `test_protocol_memory`: that harness drives
//! every trustee to a fixpoint in lockstep, so it exercises exactly one
//! interleaving. Here `stateright` explores the *tree of interleavings* — which
//! trustee advances when — and checks properties over every state it reaches.
//!
//! What makes it different from a model: nothing is modelled. Each transition
//! rehydrates a real [`BoardClient`] over a real [`MemoryBoard`], calls the real
//! [`Trustee::step`] (hence the real `datalog::composed::run` and the real
//! action layer with real cryptography), and posts through the real
//! [`BoardClient::post`] with its outgoing mailbox (§6.4). There is no second
//! rendering of the rules to drift from the first.
//!
//! # State, and why it is shaped this way
//!
//! `stateright` needs a state value it can clone, hash and compare, whereas a
//! live board client owns a transport, a persistence handle and secret keys. So
//! the state holds only what is *durable*: the board's bytes (b4 stores opaque
//! bytes anyway) plus, per trustee, its committed set and own-post record. Every
//! transition rebuilds a client from that, which is exactly a restart — the same
//! path §6.3/§6.4 make claims about. The keys live in the model, not the state,
//! since they are fixed for a run.
//!
//! # The limit worth knowing
//!
//! `Context::get_rng()` is a `ThreadRng`, so real cryptography is
//! nondeterministic: a fresh sharing, shuffle or proof yields a fresh hash every
//! time. Two interleavings that "should" converge therefore produce different
//! boards and never dedupe, so this is exploration of a **tree**, not exhaustive
//! checking of a graph — its size is the product of the per-phase orderings, and
//! it is affordable only for tiny committees. Recovering deduplication (and with
//! it larger `n`) needs the crypto abstracted to deterministic placeholder
//! hashes: a second, hash-only mode, deliberately not built yet.
//!
//! One mitigation is built in: the **lookahead**. `actions` computes each
//! candidate turn's successor (once — the results are memoized) and offers only
//! the turns that move the system; `next_state` serves the stored edge. Idle
//! trustees therefore never appear as actions, and — the part that matters —
//! `next_state` is a deterministic function of `(state, action)` despite the
//! crypto, so the paths stateright reconstructs for property discoveries and
//! counterexamples are byte-identical to the graph it checked.

mod common;

use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::Mutex;

use common::MemoryPersistence;
use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::elgamal::{KeyPair, PublicKey};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::utils::signatures::SignatureScheme;
use stateright::{Checker, Model, Property};

use braid::board::transport::{MemoryBoard, MemoryTransport, StagedRef};
use braid::board::BoardClient;
use braid::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use braid::messages::newtypes::{
    hash_bytes, ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex,
};
use braid::messages::predicate::Predicate;
use braid::messages::wire::{MessageType, ProtocolMessage};
use braid::protocol_manager::ProtocolManager;
use braid::trustee::Trustee;

type C = RistrettoCtx;
type Element = <C as Context>::Element;
type Group = <C as Context>::G;
type Sig = <C as Context>::SignatureScheme;
type Signer = <Sig as SignatureScheme<<C as Context>::Rng>>::Signer;
/// Ciphertext width, and the number of ballots. Both minimal: the crypto cost is
/// paid once per explored transition, so this is the knob that decides whether
/// the exploration finishes at all.
const W: usize = 2;
const BALLOTS: usize = 2;
const TRUSTEES: usize = 2;
const THRESHOLD: usize = 2;
/// Depth cap. Without deduplication (see the module note) the search is a tree,
/// so this is what makes it terminate. A healthy run publishes plaintexts well
/// inside it.
const MAX_DEPTH: usize = 24;

const DATE: Timestamp = 0;

thread_local! {
    /// One current-thread runtime per worker: the board client's cycle is async,
    /// `next_state` is not.
    static RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("failed to build tokio runtime");
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    RUNTIME.with(|rt| rt.block_on(f))
}

///////////////////////////////////////////////////////////////////////////
// State
///////////////////////////////////////////////////////////////////////////

/// One trustee's durable state (§6.2, §6.4), serialized so the whole state is
/// hashable.
#[derive(Clone, PartialEq, Eq, Hash)]
struct TrusteeDurable {
    committed: Vec<Vec<u8>>,
    own_posts: Vec<(Vec<u8>, String)>,
}

/// The whole system's durable state: the board plus every trustee's records.
#[derive(Clone, PartialEq, Eq, Hash)]
struct SystemState {
    /// Serialized `ProtocolMessage`s, in board order.
    board: Vec<Vec<u8>>,
    trustees: Vec<TrusteeDurable>,
    /// Datalog halts observed so far. A healthy run leaves this empty; the
    /// safety property is exactly that it stays empty.
    halts: Vec<String>,
}

impl std::fmt::Debug for SystemState {
    /// Compact on purpose: `stateright` prints states in counterexamples, and the
    /// raw bytes would bury the useful part. Shows the board as message types and
    /// each trustee as its record sizes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let types: Vec<String> = self
            .board
            .iter()
            .map(|bytes| match ProtocolMessage::<C>::deser(bytes) {
                Ok(m) => format!("{:?}", m.message_type),
                Err(_) => "??".to_string(),
            })
            .collect();
        write!(f, "board[{}]", types.join(","))?;
        for (i, t) in self.trustees.iter().enumerate() {
            write!(
                f,
                " t{}(in={},out={})",
                i + 1,
                t.committed.len(),
                t.own_posts.len()
            )?;
        }
        if !self.halts.is_empty() {
            write!(f, " HALTS={:?}", self.halts)?;
        }
        Ok(())
    }
}

/// One step of the system: whose turn it is.
///
/// A trustee action is one full update-first cycle (§6) — fetch, infer, execute,
/// post — which is the granularity at which trustees actually interleave.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Turn {
    /// Trustee `i` (0-based) runs one cycle.
    Trustee(usize),
    /// The manager encrypts the ballots and posts them, which it can only do
    /// once the DKG has published a public key.
    PostBallots,
}

///////////////////////////////////////////////////////////////////////////
// Model
///////////////////////////////////////////////////////////////////////////

/// The fixed context of a run: keys, configuration, and the plaintexts the
/// manager will encrypt. Not part of the state — none of it changes.
struct BraidModel {
    manager: ProtocolManager<C>,
    signing_keys: Vec<Signer>,
    share_keypairs: Vec<KeyPair<C>>,
    configuration: Configuration<C>,
    configuration_hash: ConfigurationHash,
    mixing_trustees: Vec<TrusteeIndex>,
    plaintexts_in: Vec<[Element; W]>,
    /// Every explored edge's successor, computed once in [`Self::lookahead`]
    /// and never recomputed. This is what makes `next_state` a deterministic
    /// function of `(state, action)` even though the crypto inside a cycle is
    /// not: exploration, property discovery and counterexample replay all see
    /// the same bytes for the same edge. Grows with the number of explored
    /// edges — trivial at this scale, a knob to revisit for larger runs.
    memo: Mutex<HashMap<(SystemState, Turn), Option<SystemState>>>,
}

impl BraidModel {
    fn new() -> Self {
        let mut key_rng = C::get_rng();
        let manager = ProtocolManager::<C>::new(Sig::gen_signing_key(&mut key_rng));

        let mut signing_keys = Vec::with_capacity(TRUSTEES);
        let mut trustee_vks = Vec::with_capacity(TRUSTEES);
        let mut share_keypairs = Vec::with_capacity(TRUSTEES);
        let mut share_enc_keys = Vec::with_capacity(TRUSTEES);
        for _ in 0..TRUSTEES {
            let sk = Sig::gen_signing_key(&mut key_rng);
            trustee_vks.push(Sig::verifying_key(&sk));
            signing_keys.push(sk);
            let keypair = KeyPair::<C>::generate();
            share_enc_keys.push(keypair.pkey.y.clone());
            share_keypairs.push(keypair);
        }

        let configuration = Configuration::<C>::new(
            0,
            Sig::verifying_key(&manager.signing_key),
            trustee_vks,
            THRESHOLD,
            W,
            share_enc_keys,
            PhantomData,
        );
        let configuration_hash =
            ConfigurationHash::from_configuration(&configuration).expect("configuration hash");

        let mut enc_rng = C::get_rng();
        let plaintexts_in: Vec<[Element; W]> = (0..BALLOTS)
            .map(|_| std::array::from_fn(|_| Group::random_element(&mut enc_rng)))
            .collect();

        Self {
            manager,
            signing_keys,
            share_keypairs,
            configuration,
            configuration_hash,
            mixing_trustees: (1..=THRESHOLD).collect(),
            plaintexts_in,
            memo: Mutex::new(HashMap::new()),
        }
    }

    /// Rebuild the shared board from a state's bytes.
    fn board_from(&self, state: &SystemState) -> std::sync::Arc<MemoryBoard<C>> {
        let board = MemoryBoard::<C>::new();
        for bytes in &state.board {
            board.push(
                ProtocolMessage::<C>::deser(bytes).expect("board holds well-formed message bytes"),
            );
        }
        board
    }

    /// Rebuild trustee `i`'s persistence from a state's records.
    fn persistence_from(&self, durable: &TrusteeDurable) -> MemoryPersistence {
        let committed = durable
            .committed
            .iter()
            .map(|b| Predicate::deser(b).expect("persisted predicate"))
            .collect();
        let own_posts = durable
            .own_posts
            .iter()
            .map(|(b, handle)| {
                (
                    Predicate::deser(b).expect("persisted own-post predicate"),
                    StagedRef(handle.clone()),
                )
            })
            .collect();
        MemoryPersistence::restored(committed, own_posts)
    }

    fn durable_from(persistence: &MemoryPersistence) -> TrusteeDurable {
        let (committed, own_posts) = persistence.snapshot();
        TrusteeDurable {
            committed: committed.iter().map(|p| p.ser()).collect(),
            own_posts: own_posts
                .iter()
                .map(|(p, staged)| (p.ser(), staged.0.clone()))
                .collect(),
        }
    }

    /// The DKG public key on the board, if the DKG has got that far.
    fn public_key_on(&self, state: &SystemState) -> Option<(DkgPublicKey<C>, PublicKeyHash)> {
        for bytes in &state.board {
            let message = ProtocolMessage::<C>::deser(bytes).ok()?;
            if message.message_type == MessageType::PublicKey {
                let body = message.body.as_ref()?;
                let pk = DkgPublicKey::<C>::deser(body).ok()?;
                return Some((pk, PublicKeyHash(hash_bytes(body))));
            }
        }
        None
    }

    /// The plaintexts published on the board, if any.
    fn plaintexts_on(&self, state: &SystemState) -> Option<Plaintexts<C, W>> {
        for bytes in &state.board {
            let message = ProtocolMessage::<C>::deser(bytes).ok()?;
            if message.message_type == MessageType::Plaintexts {
                return Plaintexts::<C, W>::deser(message.body.as_ref()?).ok();
            }
        }
        None
    }

    /// Compute the successor of `state` under `turn`: the one real transition.
    ///
    /// Called exactly once per edge, via [`Self::lookahead`]. `None` means
    /// "not a transition", for one of two distinct reasons:
    ///
    /// - **Idle cycle** (nothing produced): discarded outright, *including*
    ///   any committed-set growth its `update()` performed. This is a
    ///   deliberate compression — observation timing stays out of the state
    ///   space. It is sound while the board is honest and append-only: the
    ///   §6.3 gate can never fire, so how current a trustee's anti-rewrite
    ///   pin is cannot influence any checked behavior. Fault injection that
    ///   drops or rewrites board messages invalidates that argument, and this
    ///   clause must be revisited with it.
    /// - **Self-loop** (produced, but `next == *state`): a cycle whose net
    ///   effect is nothing — reachable only via the mailbox's commit-only
    ///   re-send path (§6.4), which an honest transport never exposes because
    ///   production always grows the board. Stateright's fingerprint dedup
    ///   would fold the duplicate anyway; returning `None` states the intent.
    fn successor(&self, state: &SystemState, turn: &Turn) -> Option<SystemState> {
        let board = self.board_from(state);
        let mut next = state.clone();

        match turn {
            Turn::Trustee(i) => {
                let i = *i;
                let persistence = self.persistence_from(&state.trustees[i]);
                let outcome = block_on(async {
                    let mut client = BoardClient::connect(
                        MemoryTransport::new(board.clone()),
                        persistence.clone(),
                    )
                    .await?;
                    let trustee = Trustee::<C>::new(
                        (i + 1).to_string(),
                        self.signing_keys[i].clone(),
                        self.share_keypairs[i].clone(),
                        client.configuration(),
                    )?;
                    client.update().await?;
                    let produced = trustee.step(client.view())?;
                    let produced_any = !produced.is_empty();
                    if produced_any {
                        client.post(produced).await?;
                    }
                    Ok::<bool, anyhow::Error>(produced_any)
                });

                match outcome {
                    Ok(produced_any) => {
                        if !produced_any {
                            return None;
                        }
                    }
                    Err(e) => next.halts.push(format!("t{}: {e:#}", i + 1)),
                }
                next.trustees[i] = Self::durable_from(&persistence);
            }
            Turn::PostBallots => {
                use cryptography::cryptosystem::naoryung;
                let (dkg_pk, pk_hash) = self.public_key_on(state)?;
                let pk = PublicKey::<C>::new(dkg_pk.pk.clone());
                let ctx_enc = braid::trustee::ballot_encryption_context::<C>(
                    self.configuration.id,
                    &dkg_pk.pk,
                );
                let ny_pk = naoryung::PublicKey::augment(&pk, &ctx_enc)
                    .expect("auxiliary key derivation cannot fail");
                let encrypted: Vec<naoryung::Ciphertext<C, W>> = self
                    .plaintexts_in
                    .iter()
                    .map(|p| ny_pk.encrypt(p, &ctx_enc))
                    .collect::<Result<_, _>>()
                    .expect("ballot encryption cannot fail");
                let ballots = Ballots::<C, W>::new(encrypted);
                board.push(ProtocolMessage::<C>::ballots(
                    &self.manager,
                    DATE,
                    self.configuration_hash,
                    pk_hash,
                    self.mixing_trustees.clone(),
                    1, // tally_id: single tally in the model
                    &ballots,
                ));
            }
        }

        next.board = board.snapshot().iter().map(|m| m.ser()).collect();
        if next == *state {
            return None;
        }
        Some(next)
    }

    /// Memoized [`Self::successor`]: compute on first sight of an edge, serve
    /// the stored result ever after. First writer wins, so an edge has one
    /// identity for its whole life — the peek from `actions` IS the evaluation
    /// that `next_state` later returns.
    fn lookahead(&self, state: &SystemState, turn: &Turn) -> Option<SystemState> {
        let key = (state.clone(), turn.clone());
        if let Some(cached) = self.memo.lock().unwrap().get(&key) {
            return cached.clone();
        }
        // Compute outside the lock: the cycle does real work, and another
        // worker asking for a different edge shouldn't wait on it.
        let computed = self.successor(state, turn);
        self.memo
            .lock()
            .unwrap()
            .entry(key)
            .or_insert(computed)
            .clone()
    }
}

impl Model for BraidModel {
    type State = SystemState;
    type Action = Turn;

    fn init_states(&self) -> Vec<Self::State> {
        let configuration_message =
            ProtocolMessage::<C>::configuration(&self.manager, DATE, &self.configuration);
        vec![SystemState {
            board: vec![configuration_message.ser()],
            trustees: (0..TRUSTEES)
                .map(|_| TrusteeDurable {
                    committed: Vec::new(),
                    own_posts: Vec::new(),
                })
                .collect(),
            halts: Vec::new(),
        }]
    }

    /// The lookahead: every candidate turn's successor is computed here (and
    /// memoized), and only turns that actually move the system are offered.
    /// Idle trustees never appear as actions — not because idleness is
    /// predicted (that would take a second rendering of the rules), but
    /// because the one evaluation the checker was going to pay anyway happens
    /// now, and its result is kept.
    fn actions(&self, state: &Self::State, actions: &mut Vec<Self::Action>) {
        // A halted system takes no further steps: the trustee stops, which is
        // what the safety property is about.
        if !state.halts.is_empty() {
            return;
        }
        let mut candidates: Vec<Turn> = (0..TRUSTEES).map(Turn::Trustee).collect();
        // The manager posts ballots once, after the DKG yields a public key.
        let has_ballots = state.board.iter().any(|bytes| {
            ProtocolMessage::<C>::deser(bytes)
                .map(|m| m.message_type == MessageType::Ballots)
                .unwrap_or(false)
        });
        if !has_ballots && self.public_key_on(state).is_some() {
            candidates.push(Turn::PostBallots);
        }
        for turn in candidates {
            if self.lookahead(state, &turn).is_some() {
                actions.push(turn);
            }
        }
    }

    fn next_state(&self, last: &Self::State, action: Self::Action) -> Option<Self::State> {
        // Serve the edge computed by `actions`. The memo hit is what makes
        // this deterministic; a miss (which plain exploration never produces)
        // computes and stores the edge the same way.
        self.lookahead(last, &action)
    }

    fn properties(&self) -> Vec<Property<Self>> {
        vec![
            // Safety, checked on every reachable state. This subsumes the
            // mailbox property (§6.4): were a trustee ever to publish a second
            // artifact for a slot it had already filled, some view would hold a
            // colliding pair and its `step` would return a datalog error, which
            // lands here.
            Property::<Self>::always("no trustee halts", |_, state| state.halts.is_empty()),
            // Non-vacuity, and the end-to-end result: some reachable state
            // publishes exactly the plaintexts the manager encrypted. Stated as
            // reachability rather than liveness because the search is depth-capped
            // (see the module note), so "on every path" is not a claim this
            // exploration can support.
            Property::<Self>::sometimes(
                "plaintexts published correctly",
                |model, state| match model.plaintexts_on(state) {
                    Some(published) => {
                        let expected: HashSet<[Element; W]> =
                            model.plaintexts_in.iter().cloned().collect();
                        let actual: HashSet<[Element; W]> = published.0.into_iter().collect();
                        expected == actual
                    }
                    None => false,
                },
            ),
        ]
    }
}

/// Explore the interleavings of a two-trustee run over the real implementation.
///
/// Ignored by default: each transition performs real DKG/shuffle/decryption
/// cryptography, so this costs orders of magnitude more than the fixpoint
/// harness. Run it explicitly:
/// `cargo test --release -p braid --test model_check -- --ignored --nocapture`
#[test]
#[ignore]
fn model_check_two_trustees() {
    let model = BraidModel::new();
    let checker = model
        .checker()
        .target_max_depth(MAX_DEPTH)
        .threads(1)
        .spawn_bfs()
        .join();
    checker.assert_properties();
    println!(
        "explored {} states, max depth {}",
        checker.unique_state_count(),
        MAX_DEPTH
    );
}
