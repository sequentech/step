// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Explicit-state model checking of the trustee protocol over the **real
//! datalog** with **symbolic artifacts** (tokens) in place of cryptography.
//!
//! The companion harness (`model_check.rs`) drives the full stack, crypto
//! included; its module docs explain why that exploration is a tree:
//! `ThreadRng` makes every artifact fresh, so logically-equal states never
//! dedupe. This harness removes exactly that obstacle and no more.
//!
//! # What is real, what is symbolic
//!
//! **Real**: the datalog ([`braid::datalog::composed::run`] — the same call
//! `Trustee::step` makes, on the same predicates), predicate extraction
//! (`verify()`), message assembly (the [`ProtocolMessage`] constructors, so
//! heads, statements and hash chaining are production code), signatures
//! (ed25519 signing is deterministic and cheap), the board client with its
//! committed set (§6.2/§6.3) and outgoing mailbox (§6.4), transport and
//! persistence.
//!
//! **Symbolic**: artifact *bodies*. Where the real action layer shuffles,
//! deals shares or decrypts, [`SymbolicModel::execute_symbolic`] fabricates a
//! deterministic token — a function of the action's hash-bound inputs, plus
//! the producer's index exactly when the real artifact would differ per
//! trustee (shares, mixes, partial decryptions) and not when it wouldn't (the
//! joint public key, the combined plaintexts, which must agree across
//! trustees). `H(token)` chains through heads and predicates exactly as a
//! real artifact hash would.
//!
//! # What this buys
//!
//! Every transition is a deterministic function of the state: message bytes
//! are path-independent (constant heads, tokens that are functions of hashes,
//! deterministic ed25519), so edges replay exactly and the terminal property
//! upgrades from the crypto harness's `sometimes` to a genuine `eventually` —
//! every path must complete, checkable soundly because exploration is
//! exhaustive, acyclic (the board only grows) and not depth-capped.
//!
//! And state identity is **order-free** ([`SystemState::canonicalize`]): the
//! board is an ordered log, but the protocol layer is order-insensitive
//! (datalog consumes predicate *sets*, stores are content-addressed), so
//! states are quotiented by message order and interleavings that produce the
//! same message set fold. Measured at n=2: 35 states, versus 153 for the same
//! configuration without the quotient — which is also exactly the crypto
//! harness's tree, a structural cross-validation of the symbolic layer
//! (identical actions, identical branching, before folding).
//!
//! # The transport model
//!
//! Unlike the crypto harness (which reuses the production `MemoryTransport`,
//! where staging IS the board append), this harness models b4's real shape
//! ([`ModelTransport`]): a **staging area** (the S3-analogue, carried in the
//! state) distinct from the **board** (committed rows), and a per-trustee
//! **view** (the board minus what b4 withholds). Fault-free this changes
//! nothing — the staging area is a deterministic function of the board and
//! nothing is withheld, measured as identical state counts — but it is the
//! seam fault actions need: crash between stage and commit, dropped commits,
//! withheld messages, split views.
//!
//! # Faults
//!
//! Faults are **actions** — never randomness inside a transition (edges must
//! stay deterministic). Each fault class is a [`Turn`] variant offered by
//! `actions()` while its **budget** ([`FaultBudgets`], model config) exceeds
//! its **spent counter** ([`FaultRecord`], in the state): bounded
//! nondeterminism, explored adversarially like everything else, so a property
//! that survives is a claim over *every* pattern of at most budget-many
//! faults. The counters double as provenance — properties can condition on
//! what has fired — and per-class `sometimes` guards (emitted only when a
//! budget enables the class) keep the machinery honest: a fault model that
//! never actually fires cannot silently pass its properties. With all budgets
//! zero the model is unchanged (measured: identical state counts).
//!
//! Implemented classes:
//! - [`Turn::DropCommit`] (benign): trustee `i` runs a full cycle whose
//!   commits never land. One action covers two stories with identical
//!   resulting state — crash after the §6.4 commit point before the send, and
//!   b4 losing an acked commit: either way the own-post record is written,
//!   the body is staged, and no board row exists. One budget unit per faulty
//!   cycle (not per message). The fault is transport-level and silent (a real
//!   `Session` retries; only datalog errors halt), and the mailbox's
//!   compute-once/send-until-acked discipline is what recovers it — so the
//!   unconditional properties must still hold: no halts, every path
//!   completes.
//!
//! # What this cannot see
//!
//! The symbolic axioms are stipulated, not checked: that honestly computed
//! artifacts verify (e.g. Fiat-Shamir domain agreement between prover and
//! verifier) and that forged ones do not. Those live in the crypto harness and
//! the crypto layer's own tests. Assuming them, everything the protocol builds
//! on top — interleaving, halting, slot collisions, chain lineage — is checked
//! here. An attack that breaks the axioms themselves is a cryptanalysis
//! result, out of scope for any model checker.

mod common;

use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Mutex;

use common::MemoryPersistence;
use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::elgamal::KeyPair;
use cryptography::utils::serialization::{VDeserializable, VSerializable};
use cryptography::utils::signatures::SignatureScheme;
use stateright::{Checker, Model, Property};

use braid::board::store::MessageStore;
use braid::board::transport::{StagedRef, Transport};
use braid::board::BoardClient;
use braid::datalog::action::Action;
use braid::messages::artifact::Configuration;
use braid::messages::newtypes::{
    hash_bytes, ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex,
};
use braid::messages::predicate::{ConfigurationValid, Predicate};
use braid::messages::wire::{MessageType, ProtocolMessage, Signer as WireSigner};
use braid::protocol_manager::ProtocolManager;

type C = RistrettoCtx;
type Sig = <C as Context>::SignatureScheme;
type SigningKey = <Sig as SignatureScheme<<C as Context>::Rng>>::Signer;

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

/// The whole system's durable state: the board, the staging area, what b4
/// withholds from whom, and every trustee's records.
#[derive(Clone, PartialEq, Eq, Hash)]
struct SystemState {
    /// Serialized `ProtocolMessage`s: what b4 has **committed** (board rows).
    board: Vec<Vec<u8>>,
    /// The staging area (S3-analogue), `(handle, message bytes)`: bodies that
    /// have been staged (§6.4), whether or not their commit has landed. In
    /// fault-free runs this is exactly the non-Configuration board content —
    /// the crash-between-stage-and-commit fault is what will make them differ.
    staged: Vec<(String, Vec<u8>)>,
    /// Per-trustee visibility: handles (see [`staged_handle`]) of board rows b4
    /// withholds from trustee `i`. Empty in fault-free runs; adversarial-board
    /// fault actions (drops, split views) populate it.
    withheld: Vec<Vec<String>>,
    trustees: Vec<TrusteeDurable>,
    /// Fault provenance: what has fired on this path (see the module's Faults
    /// section).
    faults: FaultRecord,
    /// Datalog halts observed so far. A healthy run leaves this empty; the
    /// safety property is exactly that it stays empty.
    halts: Vec<String>,
}

impl SystemState {
    /// Canonical form: state identity is **order-free**. The protocol layer is
    /// order-insensitive by design — datalog consumes predicate *sets*, the
    /// message store is content-addressed, nothing reads board positions — so
    /// interleavings that produce the same message set are the same state, and
    /// sorting is what lets stateright's fingerprint dedup see that (the tree
    /// → graph collapse). The Configuration message stays pinned first: the
    /// board client reads it at connect. Duplicate board rows (a re-committed
    /// handle leaves b4 holding two copies of identical bytes) are removed:
    /// they are protocol-identical and deduplicated on read (§8.5 Note 2), so
    /// the state identity dedups them too.
    ///
    /// Every state in the system is canonical: the initial state trivially,
    /// successors by construction in [`SymbolicModel::successor`].
    fn canonicalize(&mut self) {
        if self.board.len() > 1 {
            self.board[1..].sort_unstable();
        }
        self.board.dedup();
        self.staged.sort_unstable();
        self.staged.dedup();
        for w in &mut self.withheld {
            w.sort_unstable();
            w.dedup();
        }
        for t in &mut self.trustees {
            t.committed.sort_unstable();
            t.own_posts.sort_unstable();
        }
        self.halts.sort_unstable();
    }
}

impl std::fmt::Debug for SystemState {
    /// Compact on purpose: `stateright` prints states in counterexamples, and the
    /// raw bytes would bury the useful part.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let types: Vec<String> = self
            .board
            .iter()
            .map(|bytes| match ProtocolMessage::<C>::deser(bytes) {
                Ok(m) => format!("{:?}", m.message_type),
                Err(_) => "??".to_string(),
            })
            .collect();
        write!(f, "board[{}] staged={}", types.join(","), self.staged.len())?;
        for (i, t) in self.trustees.iter().enumerate() {
            write!(
                f,
                " t{}(in={},out={}{})",
                i + 1,
                t.committed.len(),
                t.own_posts.len(),
                if self.withheld[i].is_empty() {
                    String::new()
                } else {
                    format!(",hidden={}", self.withheld[i].len())
                }
            )?;
        }
        if self.faults.dropped_commits > 0 {
            write!(f, " drops={}", self.faults.dropped_commits)?;
        }
        if !self.halts.is_empty() {
            write!(f, " HALTS={:?}", self.halts)?;
        }
        Ok(())
    }
}

/// One step of the system: whose turn it is, and under which fault.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Turn {
    /// Trustee `i` (0-based) runs one full update/infer/post cycle.
    Trustee(usize),
    /// Trustee `i` runs a full cycle whose commits are silently lost (benign
    /// fault, budgeted): records are written and bodies staged, but nothing
    /// becomes board-visible. See the module's Faults section.
    DropCommit(usize),
    /// The manager posts the ballots (a token), which it can only do once the
    /// DKG has published a public key.
    PostBallots,
}

///////////////////////////////////////////////////////////////////////////
// Faults
///////////////////////////////////////////////////////////////////////////

/// How many faults of each class the exploration may inject (model config).
/// Zero everywhere by default: the fault-free model.
#[derive(Clone, Default)]
struct FaultBudgets {
    /// Maximum [`Turn::DropCommit`] cycles across a run.
    dropped_commits: usize,
}

/// How many faults of each class have fired on this path (state). Doubles as
/// provenance for conditioned properties and the `sometimes` guards.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
struct FaultRecord {
    dropped_commits: usize,
}

///////////////////////////////////////////////////////////////////////////
// Transport
///////////////////////////////////////////////////////////////////////////

/// A message's staging handle: hex of its content hash. Content-derived so it
/// is deterministic across paths (canonical-identity-friendly), and so
/// re-staging the same bytes maps to the same S3 object, as in production. The
/// same scheme identifies board rows in [`SystemState::withheld`].
fn staged_handle(bytes: &[u8]) -> String {
    hex::encode(&hash_bytes(bytes)[..])
}

/// The model's b4: what one actor sees and can do during one cycle.
///
/// Reads serve the actor's **view** — the true board minus whatever b4
/// withholds from it. `stage` writes the body into the staging area (the
/// S3-analogue carried in [`SystemState::staged`]) *without* touching the
/// board; `commit` looks the handle up in the staging area and appends the
/// message to the commit sink, which the harness merges into the true board
/// after the cycle (so, as with real b4, an actor's own post becomes visible
/// to it on its next fetch, not within the posting cycle).
///
/// This is the split the production `MemoryTransport` deliberately collapses
/// (staging *is* the append there — see its docs for why that is fine in M1):
/// modeling the §6.4 seam — a crash after stage, before commit — and the
/// dropped-commit fault requires the two phases to be genuinely distinct.
/// Committing an unknown handle is an error: the staged body is gone, which is
/// the (out-of-model, §6.2-trust-class) S3-loss scenario.
struct ModelTransport<C: Context> {
    /// The messages this actor can see, Configuration included.
    view: Vec<ProtocolMessage<C>>,
    /// The staging area: seeded from the state, grown by `stage`.
    staged: Rc<RefCell<HashMap<String, Vec<u8>>>>,
    /// Messages made board-visible during this cycle, in commit order.
    committed: Rc<RefCell<Vec<ProtocolMessage<C>>>>,
    /// [`Turn::DropCommit`]: commits validate but never land, silently — the
    /// client believes b4 has the message. Staging is unaffected.
    drop_commits: bool,
}

#[async_trait::async_trait(?Send)]
impl<C: Context> Transport<C> for ModelTransport<C> {
    async fn fetch_configuration(&self) -> anyhow::Result<ProtocolMessage<C>> {
        self.view
            .iter()
            .find(|m| m.message_type == MessageType::Configuration)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("board has no Configuration message"))
    }

    async fn fetch(&self) -> anyhow::Result<Vec<ProtocolMessage<C>>> {
        Ok(self
            .view
            .iter()
            .filter(|m| m.message_type != MessageType::Configuration)
            .cloned()
            .collect())
    }

    async fn stage(&self, message: &ProtocolMessage<C>) -> anyhow::Result<StagedRef> {
        let bytes = message.ser();
        let handle = staged_handle(&bytes);
        self.staged.borrow_mut().insert(handle.clone(), bytes);
        Ok(StagedRef(handle))
    }

    async fn commit(&self, staged: &StagedRef) -> anyhow::Result<()> {
        let bytes = self
            .staged
            .borrow()
            .get(&staged.0)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!("commit of unknown staged handle {} (body lost?)", staged.0)
            })?;
        let message = ProtocolMessage::<C>::deser(&bytes)
            .map_err(|e| anyhow::anyhow!("staged bytes do not decode: {e:?}"))?;
        if !self.drop_commits {
            self.committed.borrow_mut().push(message);
        }
        Ok(())
    }
}

///////////////////////////////////////////////////////////////////////////
// The symbolic trustee
///////////////////////////////////////////////////////////////////////////

/// A trustee reduced to what the symbolic executor needs: an identity to sign
/// with and the self-scoped `ConfigurationValid` fact (§9.7) injected at every
/// step — the same two things `Trustee` carries besides its share-decryption
/// keypair, which only the crypto layer uses.
struct SymbolicTrustee {
    name: String,
    signing_key: SigningKey,
    configuration_valid: ConfigurationValid,
}

impl WireSigner<C> for SymbolicTrustee {
    fn get_signing_key(&self) -> &SigningKey {
        &self.signing_key
    }
    fn get_name(&self) -> String {
        self.name.clone()
    }
}

///////////////////////////////////////////////////////////////////////////
// Model
///////////////////////////////////////////////////////////////////////////

/// The fixed context of a run: identities and configuration. Not part of the
/// state — none of it changes.
struct SymbolicModel {
    n: usize,
    budgets: FaultBudgets,
    manager: ProtocolManager<C>,
    trustees: Vec<SymbolicTrustee>,
    configuration: Configuration<C>,
    configuration_hash: ConfigurationHash,
    mixing_trustees: Vec<TrusteeIndex>,
    /// Every explored edge's successor, computed once in [`Self::lookahead`].
    /// With symbolic artifacts the edges are deterministic anyway; the memo
    /// still saves the duplicate evaluation between `actions` and
    /// `next_state`, and now pays off across converging paths too (a memo hit
    /// on a state reached twice).
    memo: Mutex<HashMap<(SystemState, Turn), Option<SystemState>>>,
}

impl SymbolicModel {
    fn new(n: usize, threshold: usize) -> Self {
        let mut key_rng = C::get_rng();
        let manager = ProtocolManager::<C>::new(Sig::gen_signing_key(&mut key_rng));

        let mut signing_keys = Vec::with_capacity(n);
        let mut trustee_vks = Vec::with_capacity(n);
        let mut share_enc_keys = Vec::with_capacity(n);
        for _ in 0..n {
            let sk = Sig::gen_signing_key(&mut key_rng);
            trustee_vks.push(Sig::verifying_key(&sk));
            signing_keys.push(sk);
            // Present in the configuration but never used: share encryption is
            // crypto-layer machinery the symbolic executor bypasses.
            share_enc_keys.push(KeyPair::<C>::generate().pkey.y.clone());
        }

        let configuration = Configuration::<C>::new(
            0,
            Sig::verifying_key(&manager.signing_key),
            trustee_vks,
            threshold,
            2,
            share_enc_keys,
            PhantomData,
        );
        let configuration_hash = ConfigurationHash::from_configuration(&configuration)
            .expect("configuration hash");

        let trustees = signing_keys
            .into_iter()
            .enumerate()
            .map(|(i, signing_key)| {
                // 1-based trustee index (§4.3), as `Trustee::new` derives it.
                let self_index: TrusteeIndex = i + 1;
                SymbolicTrustee {
                    name: self_index.to_string(),
                    signing_key,
                    configuration_valid: ConfigurationValid {
                        configuration: configuration_hash,
                        threshold: configuration.threshold,
                        trustee_count: configuration.trustees.len(),
                        self_index,
                    },
                }
            })
            .collect();

        Self {
            n,
            budgets: FaultBudgets::default(),
            manager,
            trustees,
            configuration,
            configuration_hash,
            mixing_trustees: (1..=threshold).collect(),
            memo: Mutex::new(HashMap::new()),
        }
    }

    /// Allow up to `budget` [`Turn::DropCommit`] cycles in the exploration.
    fn with_dropped_commit_budget(mut self, budget: usize) -> Self {
        self.budgets.dropped_commits = budget;
        self
    }

    /// The board as trustee `i` sees it: the true board minus rows b4 withholds
    /// from it. Fault-free, `withheld[i]` is empty and this is the whole board.
    fn visible_board(&self, state: &SystemState, i: usize) -> Vec<ProtocolMessage<C>> {
        state
            .board
            .iter()
            .filter(|bytes| !state.withheld[i].contains(&staged_handle(bytes)))
            .map(|bytes| {
                ProtocolMessage::<C>::deser(bytes).expect("board holds well-formed message bytes")
            })
            .collect()
    }

    /// A transport for one actor's cycle, plus the handles the harness reads
    /// back afterwards: the staging area (seeded from the state) and the
    /// commit sink.
    #[allow(clippy::type_complexity)]
    fn transport_for(
        &self,
        state: &SystemState,
        view: Vec<ProtocolMessage<C>>,
        drop_commits: bool,
    ) -> (
        ModelTransport<C>,
        Rc<RefCell<HashMap<String, Vec<u8>>>>,
        Rc<RefCell<Vec<ProtocolMessage<C>>>>,
    ) {
        let staged: Rc<RefCell<HashMap<String, Vec<u8>>>> =
            Rc::new(RefCell::new(state.staged.iter().cloned().collect()));
        let committed: Rc<RefCell<Vec<ProtocolMessage<C>>>> = Rc::new(RefCell::new(Vec::new()));
        let transport = ModelTransport {
            view,
            staged: Rc::clone(&staged),
            committed: Rc::clone(&committed),
            drop_commits,
        };
        (transport, staged, committed)
    }

    /// Merge a cycle's transport effects into the successor state: the grown
    /// staging area, and the committed messages appended to the board (order
    /// is irrelevant — `canonicalize` runs before the state is used).
    fn merge_transport(
        next: &mut SystemState,
        staged: Rc<RefCell<HashMap<String, Vec<u8>>>>,
        committed: Rc<RefCell<Vec<ProtocolMessage<C>>>>,
    ) {
        next.staged = staged
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        next.board
            .extend(committed.borrow().iter().map(|m| m.ser()));
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

    /// The hash of the published DKG public key, if the DKG has got that far.
    /// No body deserialization: symbolically the hash IS the artifact.
    fn public_key_hash_on(&self, state: &SystemState) -> Option<PublicKeyHash> {
        for bytes in &state.board {
            let message = ProtocolMessage::<C>::deser(bytes).ok()?;
            if message.message_type == MessageType::PublicKey {
                return Some(PublicKeyHash(hash_bytes(message.body.as_ref()?)));
            }
        }
        None
    }

    /// `Trustee::step`, with the real inference and a symbolic action layer:
    /// same predicates, same `datalog::composed::run`, token execution.
    fn symbolic_step(
        &self,
        i: usize,
        view: &MessageStore<C>,
    ) -> anyhow::Result<Vec<ProtocolMessage<C>>> {
        let mut predicates = view.get_predicates();
        predicates.push(self.trustees[i].configuration_valid.clone().into());

        let actions =
            braid::datalog::composed::run(&predicates).map_err(|e| anyhow::anyhow!(e))?;

        let mut outgoing = Vec::new();
        for action in &actions {
            outgoing.extend(self.execute_symbolic(i, action));
        }
        Ok(outgoing)
    }

    /// Execute one derived action symbolically: assemble the real wire message
    /// around a token body.
    ///
    /// Token discipline: a token is a deterministic function of the action's
    /// hash-bound inputs; the producer's index is included exactly when the
    /// real artifact would differ per trustee (shares, mixes, partial
    /// decryptions carry per-trustee randomness or key shares) and omitted
    /// when the real computation must agree across trustees (the joint public
    /// key, the combined plaintexts) — otherwise hash-equality agreement
    /// between trustees would spuriously fail.
    fn execute_symbolic(&self, i: usize, action: &Action) -> Vec<ProtocolMessage<C>> {
        let t = &self.trustees[i];
        match action {
            Action::ComputeShares(cfg, self_index) => {
                let token = format!("shares:t{self_index}").into_bytes();
                vec![ProtocolMessage::<C>::shares(t, DATE, *cfg, &token)]
            }
            Action::ComputePublicKey(cfg, shares_hashes, _self_index) => {
                let token = format!("pk:{shares_hashes:?}").into_bytes();
                vec![ProtocolMessage::<C>::public_key(t, DATE, *cfg, &token)]
            }
            Action::ComputeMix(cfg, pk, _source, input, self_index) => {
                let token = format!("mix:t{self_index}:{input:?}").into_bytes();
                vec![ProtocolMessage::<C>::mix(t, DATE, *cfg, *pk, *input, &token)]
            }
            Action::SignMix(cfg, pk, _source, input, output, _self_index) => {
                // The real executor verifies the shuffle proof here. In this
                // fault-free milestone every artifact is honestly fabricated,
                // so "the proof verifies" holds by the symbolic axiom and the
                // signature is unconditional. Fault modeling replaces this
                // with a check of the token's validity claim.
                vec![ProtocolMessage::<C>::mix_signature(
                    t, DATE, *cfg, *pk, *input, *output,
                )]
            }
            Action::ComputePartialDecryptions(cfg, pk, cts, _shares_hashes, self_index) => {
                let token = format!("pdec:t{self_index}:{cts:?}").into_bytes();
                vec![ProtocolMessage::<C>::partial_decryptions(
                    t, DATE, *cfg, *pk, *cts, &token,
                )]
            }
            Action::ComputePlaintexts(cfg, pk, cts, pdec_hashes, _self_index) => {
                let token = format!("plain:{cts:?}:{pdec_hashes:?}").into_bytes();
                vec![ProtocolMessage::<C>::plaintexts(
                    t, DATE, *cfg, *pk, *cts, &token,
                )]
            }
        }
    }

    /// Compute the successor of `state` under `turn` — the one real transition.
    /// Same two-clause guard as the crypto harness: idle cycles are discarded
    /// before durable updates (observation-timing compression), productive
    /// cycles only if they changed nothing (`next == *state`).
    /// One trustee's full update/infer/post cycle against the (possibly
    /// fault-configured) transport, its effects merged into `next`. `None`
    /// means the cycle was idle — not a transition (see the guard notes).
    fn trustee_cycle(
        &self,
        state: &SystemState,
        next: &mut SystemState,
        i: usize,
        drop_commits: bool,
    ) -> Option<()> {
        let persistence = self.persistence_from(&state.trustees[i]);
        let (transport, staged, committed) =
            self.transport_for(state, self.visible_board(state, i), drop_commits);
        let outcome = block_on(async {
            let mut client = BoardClient::connect(transport, persistence.clone()).await?;
            client.update().await?;
            let produced = self.symbolic_step(i, client.view())?;
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
        Self::merge_transport(next, staged, committed);
        Some(())
    }

    fn successor(&self, state: &SystemState, turn: &Turn) -> Option<SystemState> {
        let mut next = state.clone();

        match turn {
            Turn::Trustee(i) => {
                self.trustee_cycle(state, &mut next, *i, false)?;
            }
            Turn::DropCommit(i) => {
                self.trustee_cycle(state, &mut next, *i, true)?;
                next.faults.dropped_commits += 1;
            }
            Turn::PostBallots => {
                let pk_hash = self.public_key_hash_on(state)?;
                let token = format!("ballots:{pk_hash:?}").into_bytes();
                let message = ProtocolMessage::<C>::ballots(
                    &self.manager,
                    DATE,
                    self.configuration_hash,
                    pk_hash,
                    self.mixing_trustees.clone(),
                    &token,
                );
                // The manager keeps no own-post record (Transport::publish =
                // stage + commit), but its message takes the same staged path.
                let (transport, staged, committed) = self.transport_for(state, Vec::new(), false);
                block_on(transport.publish(&message))
                    .expect("fault-free manager publish cannot fail");
                Self::merge_transport(&mut next, staged, committed);
            }
        }

        next.canonicalize();
        if next == *state {
            return None;
        }
        Some(next)
    }

    /// Memoized [`Self::successor`], as in the crypto harness.
    fn lookahead(&self, state: &SystemState, turn: &Turn) -> Option<SystemState> {
        let key = (state.clone(), turn.clone());
        if let Some(cached) = self.memo.lock().unwrap().get(&key) {
            return cached.clone();
        }
        let computed = self.successor(state, turn);
        self.memo.lock().unwrap().entry(key).or_insert(computed).clone()
    }
}

impl Model for SymbolicModel {
    type State = SystemState;
    type Action = Turn;

    fn init_states(&self) -> Vec<Self::State> {
        let configuration_message =
            ProtocolMessage::<C>::configuration(&self.manager, DATE, &self.configuration);
        vec![SystemState {
            board: vec![configuration_message.ser()],
            staged: Vec::new(),
            withheld: vec![Vec::new(); self.n],
            trustees: (0..self.n)
                .map(|_| TrusteeDurable {
                    committed: Vec::new(),
                    own_posts: Vec::new(),
                })
                .collect(),
            faults: FaultRecord::default(),
            halts: Vec::new(),
        }]
    }

    /// The lookahead: only turns that actually move the system are offered.
    fn actions(&self, state: &Self::State, actions: &mut Vec<Self::Action>) {
        // A halted system takes no further steps.
        if !state.halts.is_empty() {
            return;
        }
        let mut candidates: Vec<Turn> = (0..self.n).map(Turn::Trustee).collect();
        // Fault turns, while their budget lasts. The lookahead prunes the
        // pointless ones for free: a faulty cycle of an idle trustee produces
        // nothing and is not a transition.
        if state.faults.dropped_commits < self.budgets.dropped_commits {
            candidates.extend((0..self.n).map(Turn::DropCommit));
        }
        // The manager posts ballots once, after the DKG yields a public key.
        let has_ballots = state.board.iter().any(|bytes| {
            ProtocolMessage::<C>::deser(bytes)
                .map(|m| m.message_type == MessageType::Ballots)
                .unwrap_or(false)
        });
        if !has_ballots && self.public_key_hash_on(state).is_some() {
            candidates.push(Turn::PostBallots);
        }
        for turn in candidates {
            if self.lookahead(state, &turn).is_some() {
                actions.push(turn);
            }
        }
    }

    fn next_state(&self, last: &Self::State, action: Self::Action) -> Option<Self::State> {
        self.lookahead(last, &action)
    }

    fn properties(&self) -> Vec<Property<Self>> {
        let mut props = vec![
            // Safety, on every reachable state — UNCONDITIONAL over the
            // benign-fault space: no pattern of at most budget-many dropped
            // commits may ever halt a trustee. (Adversarial fault classes,
            // when added, get conditioned variants instead.)
            Property::<Self>::always("no trustee halts", |_, state| state.halts.is_empty()),
            // Liveness, in its strong form: on EVERY path the protocol
            // completes. Sound here, unlike in the crypto harness, because the
            // exploration is exhaustive (deterministic edges + dedup) and
            // acyclic (the board only grows), with no depth cap. Also
            // UNCONDITIONAL over the benign-fault space — this is the k-fault-
            // tolerance claim: the mailbox's send-until-acked discipline
            // recovers every dropped commit.
            //
            // Completion is plaintexts from every MIXING trustee, not every
            // trustee: the post-DKG quorum is the mixing list (of size ==
            // threshold) — both `ComputePartialDecryptions` and
            // `ComputePlaintexts` require `mixing_position` (decrypt.rs).
            // Non-mixing trustees go quiet after the DKG by design.
            Property::<Self>::eventually("protocol completes", |model, state| {
                plaintexts_on(state) == model.mixing_trustees.len()
            }),
        ];
        // Non-vacuity guards, emitted only when a budget enables the class: a
        // fault model that never fires would otherwise pass everything above
        // without testing anything.
        if self.budgets.dropped_commits > 0 {
            props.push(Property::<Self>::sometimes(
                "a commit is dropped",
                |_, state| state.faults.dropped_commits > 0,
            ));
            props.push(Property::<Self>::sometimes(
                "the protocol completes despite a dropped commit",
                |model, state| {
                    state.faults.dropped_commits > 0
                        && plaintexts_on(state) == model.mixing_trustees.len()
                },
            ));
        }
        props
    }
}

/// The number of `Plaintexts` messages on the board.
fn plaintexts_on(state: &SystemState) -> usize {
    state
        .board
        .iter()
        .filter(|bytes| {
            ProtocolMessage::<C>::deser(bytes)
                .map(|m| m.message_type == MessageType::Plaintexts)
                .unwrap_or(false)
        })
        .count()
}

/// Explore ALL interleavings over the real datalog with symbolic artifacts.
/// Not `#[ignore]`d: with tokens instead of crypto this is meant to be fast
/// enough for the ordinary test suite — that speed is part of what the test
/// demonstrates.
fn check(model: SymbolicModel, label: &str) -> usize {
    let checker = model.checker().threads(1).spawn_bfs().join();
    checker.assert_properties();
    let states = checker.unique_state_count();
    println!("{label}: explored {states} unique states");
    states
}

#[test]
fn model_check_symbolic_two_trustees() {
    check(SymbolicModel::new(2, 2), "n=2 t=2");
}

#[test]
fn model_check_symbolic_three_trustees() {
    check(SymbolicModel::new(3, 3), "n=3 t=3");
}

#[test]
fn model_check_symbolic_three_trustees_threshold_two() {
    check(SymbolicModel::new(3, 2), "n=3 t=2");
}

/// The k-fault-tolerance run: every pattern of at most 2 dropped commits, at
/// every reachable point, interleaved every possible way — no halts, every
/// path still completes, and the guards confirm the faults actually fired.
#[test]
fn model_check_symbolic_two_trustees_dropped_commits() {
    check(
        SymbolicModel::new(2, 2).with_dropped_commit_budget(2),
        "n=2 t=2 drops<=2",
    );
}
