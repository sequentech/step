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
//! Ballots and mix tokens additionally carry **symbolic content** (see the
//! Symbolic content section): a ballot set is a multiset of voter symbols, and
//! a mix carries a [`Transform`] describing its effect on that multiset. This
//! is what lets properties assert privacy and integrity directly — about what
//! is decrypted, not about the mechanisms that were supposed to protect it.
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
//! - [`Turn::CrashBeforeRecord`] (benign): the other side of the §6.4 commit
//!   point — the process dies after staging, before the own-post record.
//!   Injected through [`CrashingPersistence`] so the real `BoardClient::post`
//!   executes the real order and aborts exactly where a crash would: stage
//!   landed, record absent, commit never attempted, the committed-set growth
//!   from `update` kept (a crash preserves prior durable writes). The harness
//!   reads the [`CRASH_SENTINEL`] as death-not-halt. Recovery is
//!   *recomputation* (nothing pins the slot), the designed counterpart to
//!   DropCommit's re-send — so the same unconditional properties must hold.
//! - [`Turn::EquivocateBallots`] (ADVERSARIAL): the manager posts a second,
//!   different ballots message — the differencing-attack move (decrypt two
//!   input sets differing by one voter's ballot; the multiset difference of
//!   the outputs discloses that vote, with no crypto broken and no dishonest
//!   trustee). The defense under test is the `Ballots` slot's GLOBAL
//!   collision (`predicate.rs`: any two ballots collide, sender
//!   notwithstanding) → datalog error → halt, per trustee, when its OWN view
//!   holds both. Halting is per-trustee ([`SystemState::halted`]) precisely
//!   so the checker can hunt the bad interleaving — a trustee decrypting a
//!   second strand before observing the collision — which the privacy/
//!   differencing property would catch. The budget here bounds a
//!   content-creating action for finiteness; it is not a tolerance claim.
//! - **Dishonest mixer** (ADVERSARIAL, [`SymbolicModel::dishonest_mixers`]): a
//!   sub-threshold set of trustees that subvert their shuffle — honest
//!   everywhere else. Two kinds ([`DishonestKind`]): `KnownPermutation` (a
//!   valid shuffle the adversary can invert — attacks privacy/linkage),
//!   `Forge` (a shuffle that alters the ballot set — attacks integrity), and
//!   `SkipsAnchor` (mixes honestly but skips the input-ballot check — attacks
//!   privacy via split views). The defense against forgery is honest
//!   verification: an honest trustee checks that a mix's output multiset equals
//!   its input's before signing, so a forged mix never gathers the threshold
//!   signatures it needs. Against a known permutation the defense is the
//!   remaining honest mixer's opaque layer. Against an illegitimate ballot set
//!   the defense is the **anchor check** (below). Fixed for the run, not
//!   budgeted.
//! - **The input-ballot anchor** (a modeled honest behavior, not a fault): the
//!   mixnet cannot establish which ballots are legitimate — braid's trustees
//!   process whatever the manager signs. So legitimacy is an external,
//!   publicly-verifiable fact (the anchor, [`HONEST_VOTERS`]), and honest
//!   trustees enforce it by refusing a *first* mix rooted at any other ballots.
//!   The check lives at exactly the first-mix engagement — `ComputeMix` for the
//!   first mixer (its compute IS its signature), `SignMix` for the other quorum
//!   members — and nowhere else (downstream trusts the chain), mirroring where
//!   the real datalog roots and signs the chain. It is threshold-robust: an
//!   illegitimate strand needs its WHOLE quorum to skip the check. This is an
//!   honest-behavior *assumption* licensed by the trust model (§ Trust model in
//!   the spec: input legitimacy is an external precondition); the negative
//!   control (`SkipsAnchor` quorum, or the anchor removed) shows it is
//!   load-bearing.
//! - [`Turn::Withhold`] (ADVERSARIAL b4, budgeted): b4 begins hiding a board
//!   row from one trustee. This is the whole of adversarial b4 — it cannot
//!   forge (signatures are checked) and reordering is neutralized by canonical
//!   identity, so withhold/reveal is all it has. Timing does the work: hide a
//!   row a trustee has already pinned → its next `update()` finds a committed
//!   predicate missing → §6.3 gate HALT (anti-rewrite); hide one it has not
//!   pinned → it never sees it (a stutter). Split views are just different
//!   `withheld` sets per trustee. Two attack shapes emerge under the search:
//!   *type 1* (equivocation + rewrite — one trustee walked down both strands
//!   over time, defeated by §6.3 in + §6.4 out pinning) and *type 2* (split
//!   view — each trustee sees one strand, no rewrite; only constructible when
//!   two disjoint mixing quorums exist, i.e. 2·threshold ≤ n, so n=2/t=2
//!   cannot mount it). Enabling this budget also turns OFF the
//!   observation-timing compression (ruling 1) so pins are faithful.
//!
//! Adversarial classes get *conditioned* liveness (completion is not promised
//! once an adversary acts) and *conditioned* safety (only an adversary may
//! cause a halt), while the functional privacy and integrity properties (see
//! Properties) hold unconditionally.
//!
//! Deliberately NOT fault classes:
//! - **Benign fetch failures** (b4 unreachable on the read path) are
//!   stutter-equivalent: `BoardClient::update` fetches everything before
//!   admitting anything, so a read failure aborts the cycle with zero durable
//!   footprint — behaviorally identical to the trustee not being scheduled,
//!   which the exploration already quantifies over. "The system recovers from
//!   any finite number of fetch failures" is therefore a corollary of
//!   `eventually completes` over all schedules, not a claim needing its own
//!   budget (which would only multiply the graph by counter-copies of
//!   stutters). The lemma is pinned by
//!   [`fetch_failure_has_no_durable_footprint`]; if `update` ever loses its
//!   atomic-abort structure, that test is what breaks. A fetch that fails
//!   *forever* is permanent starvation — b4 denying availability, trivially
//!   within an untrusted b4's power and outside the liveness claims.
//! - **Duplicate delivery** is absorbed by the model: board rows dedup
//!   (§8.5 Note 2 read-side dedup, mirrored in `canonicalize`) and the store
//!   is a set keyed by predicate, so a repeated message is a no-op.
//! - **Withholding** (a fetch that succeeds but omits rows) has no benign
//!   version: b4 is one server over one consistent store — a fetch returns
//!   the board or an error. Serving partial or divergent boards is
//!   *adversarial* behavior (split views), handled in the adversarial tier.
//!
//! # Properties
//!
//! The properties are **functional**: they assert the assets — privacy and
//! integrity — over the symbolic content, not the mechanisms that defend them.
//! A violation is a real failure however it was reached, so one property
//! covers every way it could break.
//!
//!   * **Privacy (differencing)**: all threshold-decrypted sets carry the same
//!     ballot multiset. Two *different* decrypted sets is the differencing
//!     attack consummated (their difference discloses a ballot).
//!   * **Privacy (linkage)**: every decrypted set passed through ≥1 opaque
//!     (honest) shuffle, so the adversary cannot know the whole permutation.
//!   * **Integrity**: published plaintexts carry exactly the honest ballots — a
//!     corrupted mix never reaches decryption.
//!
//! All three are unconditional over the entire fault space (the no-exemption
//! pattern). Alongside them the harness keeps the benign-tier completion and
//! no-halt claims, the adversarial conditioned variants, and the non-vacuity
//! guards (see Faults). Full privacy *as secrecy* — what an adversary can
//! deduce — is a knowledge property no explicit-state checker expresses; these
//! are its behavioral shadows, which are what a consistent-board-plus-faults
//! model can actually decide.
//!
//! # What this cannot see
//!
//! The symbolic axioms are stipulated, not checked: that honestly computed
//! artifacts verify (e.g. Fiat-Shamir domain agreement between prover and
//! verifier) and that forged ones do not. Those live in the crypto harness and
//! the crypto layer's own tests. Assuming them, everything the protocol builds
//! on top — interleaving, halting, slot collisions, privacy and integrity of
//! the decrypted content — is checked here. An attack that breaks the axioms
//! themselves is a cryptanalysis result, out of scope for any model checker.

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
use braid::board::verify::verify;
use braid::board::BoardClient;
use braid::datalog::action::{Action, MixSource};
use braid::messages::artifact::Configuration;
use braid::messages::newtypes::{
    hash_bytes, CiphertextsHash, ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex,
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
    /// Per-trustee visibility: the **strands** b4 withholds from trustee `i`,
    /// each identified by its ballots-root hash (hex). b4 hides a strand
    /// whole — its ballots and everything derived from it — so each trustee
    /// always sees a *coherent* partial board (a split view), never an orphan
    /// mix whose root is missing. The shared DKG artifacts (shares, public key)
    /// belong to no strand and are never withheld. Empty in fault-free runs.
    withheld: Vec<Vec<String>>,
    trustees: Vec<TrusteeDurable>,
    /// Fault provenance: what has fired on this path (see the module's Faults
    /// section).
    faults: FaultRecord,
    /// Why each trustee has halted, if it has (`None` = still running). Halting
    /// is per-trustee: a halted trustee takes no further turns, the others run
    /// on — freezing the whole system at the first halt would hide exactly the
    /// interleavings the adversarial properties are about.
    ///
    /// The *reason* is a stable [`HaltReason`], NOT the error message: the raw
    /// text embeds artifact hashes in a hash-sorted order, which would leak
    /// into state identity as dedup noise. The reason distinguishes the two
    /// halt mechanisms (§5.3) — the completeness gate (§6.3, anti-rewrite) and
    /// the collision rule (§5.2, halt-on-equivocation) — so a property can
    /// name which one fired.
    halted: Vec<Option<HaltReason>>,
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
        if self.faults.crashes_before_record > 0 {
            write!(f, " crashes={}", self.faults.crashes_before_record)?;
        }
        if self.faults.ballots_equivocations > 0 {
            write!(f, " equivocations={}", self.faults.ballots_equivocations)?;
        }
        for (i, h) in self.halted.iter().enumerate() {
            if let Some(reason) = h {
                write!(f, " t{}-HALTED({reason:?})", i + 1)?;
            }
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
    /// Trustee `i` runs a cycle that dies mid-post, after staging and before
    /// the own-post record (benign fault, budgeted): the other side of the
    /// §6.4 commit point — recovery is recomputation, not re-send. See the
    /// module's Faults section.
    CrashBeforeRecord(usize),
    /// b4 begins withholding a whole strand (identified by its ballots-root
    /// hash, hex) from trustee `i` (ADVERSARIAL b4, budgeted). If `i` had
    /// pinned anything in that strand, its next `update()` finds a committed
    /// predicate missing → completeness-gate HALT (§6.3); if not, `i` simply
    /// never sees the strand → a stutter. Split views are different withheld
    /// strands per trustee. See the module's Faults section.
    Withhold(usize, String),
    /// The manager posts the ballots (a token), which it can only do once the
    /// DKG has published a public key.
    PostBallots,
    /// The ADVERSARIAL manager posts a second, different ballots message for
    /// the same configuration (same heads, different ciphertext-set token):
    /// the differencing-attack move. The `Ballots` slot is global — any two
    /// ballots predicates collide — so every trustee that acts on a view
    /// holding both must halt. See the module's Faults section.
    EquivocateBallots,
}

///////////////////////////////////////////////////////////////////////////
// Faults
///////////////////////////////////////////////////////////////////////////

/// Which of braid's two halt mechanisms (§5.3) stopped a trustee. Stable (no
/// hashes) so it can live in state identity without adding dedup noise.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum HaltReason {
    /// The completeness gate (§6.3): a predicate the trustee had committed to
    /// is no longer reconstructible from b4 — the anti-rewrite defense.
    CompletenessGate,
    /// The collision rule (§5.2): two messages projecting to the same slot in
    /// one view — halt-on-equivocation.
    Collision,
    /// Any other datalog error (a structural rule).
    Other,
}

/// How many faults of each class the exploration may inject (model config).
/// Zero everywhere by default: the fault-free model.
#[derive(Clone, Default)]
struct FaultBudgets {
    /// Maximum [`Turn::DropCommit`] cycles across a run.
    dropped_commits: usize,
    /// Maximum [`Turn::CrashBeforeRecord`] cycles across a run.
    crashes_before_record: usize,
    /// Maximum [`Turn::EquivocateBallots`] posts across a run (adversarial;
    /// this budget is a finiteness bound on a content-creating action, not a
    /// tolerance claim — see the module's Faults section).
    ballots_equivocations: usize,
    /// Maximum [`Turn::Withhold`] events across a run (adversarial b4; a
    /// finiteness bound). Enabling it also turns OFF the observation-timing
    /// compression (ruling 1) so committed-set pins are tracked faithfully and
    /// the §6.3 gate fires exactly where the real client's would.
    withholdings: usize,
}

/// How many faults of each class have fired on this path (state). Doubles as
/// provenance for conditioned properties and the `sometimes` guards.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
struct FaultRecord {
    dropped_commits: usize,
    crashes_before_record: usize,
    ballots_equivocations: usize,
    withholdings: usize,
}

/// The fault, if any, injected into one trustee cycle.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CycleFault {
    None,
    /// Commits validate but never land ([`Turn::DropCommit`]).
    DropCommits,
    /// The process dies after staging, before the own-post record is written
    /// ([`Turn::CrashBeforeRecord`]): `persist_own_post` fails with
    /// [`CRASH_SENTINEL`], aborting the real `BoardClient::post` mid-algorithm
    /// — stage landed, record didn't, commit never reached.
    CrashBeforeRecord,
}

/// Marks an injected crash (as opposed to a real datalog error, which halts).
const CRASH_SENTINEL: &str = "model-crash: died before writing the own-post record";

/// [`MemoryPersistence`] with an injectable §6.4-seam failure: transparent
/// delegate unless `fail_record` is set, in which case the own-post record
/// write — the commit point — fails with [`CRASH_SENTINEL`]. Everything
/// already persisted (the committed set grown during `update`) stays, exactly
/// like a real crash: durable state reflects writes made so far.
struct CrashingPersistence {
    inner: MemoryPersistence,
    fail_record: bool,
}

#[async_trait::async_trait(?Send)]
impl braid::board::persistence::Persistence for CrashingPersistence {
    async fn load(&self) -> anyhow::Result<Vec<Predicate>> {
        self.inner.load().await
    }
    async fn persist(&mut self, predicate: &Predicate) -> anyhow::Result<()> {
        self.inner.persist(predicate).await
    }
    async fn load_own_posts(&self) -> anyhow::Result<Vec<(Predicate, StagedRef)>> {
        self.inner.load_own_posts().await
    }
    async fn persist_own_post(
        &mut self,
        predicate: &Predicate,
        staged: &StagedRef,
    ) -> anyhow::Result<()> {
        if self.fail_record {
            return Err(anyhow::anyhow!(CRASH_SENTINEL));
        }
        self.inner.persist_own_post(predicate, staged).await
    }
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

/// The stable key (full hash, hex) identifying a strand by its ballots root.
fn strand_key(root: &CiphertextsHash) -> String {
    hex::encode(root.0)
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
    /// Test-only knob for the stutter lemma
    /// ([`fetch_failure_has_no_durable_footprint`]): `fetch` fails as if b4
    /// were unreachable. Deliberately NOT a fault action — see the module's
    /// Faults section.
    fail_fetch: bool,
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
        if self.fail_fetch {
            return Err(anyhow::anyhow!("model: b4 unavailable on fetch"));
        }
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
// Symbolic content
///////////////////////////////////////////////////////////////////////////
//
// The privacy and integrity properties reason about *what the ballots carry*
// and *how they were shuffled*, not about protocol mechanisms. Ballots and
// mixes carry a symbolic content descriptor in their token body; properties
// re-derive it from board bytes and walk the mix chain — never touching real
// crypto.
//
//   * A ballot set is a multiset of **voter symbols** (small integers). An
//     honest shuffle is invisible at the multiset level: privacy is about the
//     multiset, not the order.
//   * A mix carries the multiset it output and whether it added an **opaque**
//     (adversary-unknown) layer. An honest shuffle preserves the multiset and
//     is opaque; a dishonest mixer may instead apply a permutation it knows
//     (multiset preserved, NOT opaque) or forge the content (multiset
//     changed). The multiset it claims is what an honest verifier checks
//     against the input before signing (a real shuffle proof proves exactly
//     "output is a permutation of input" — nothing about secrecy).

/// The **anchor**: the legitimate ballot set. The mixnet cannot establish
/// input-ballot legitimacy itself (its trustees process whatever the manager
/// signs); legitimacy is an external, publicly-verifiable fact, and honest
/// trustees enforce it by refusing to process a first mix rooted at any other
/// ballots (see the anchor check in `execute_symbolic`). Two distinct voters,
/// small on purpose.
const HONEST_VOTERS: [u8; 2] = [0, 1];

/// The ballot set an equivocating manager substitutes — the honest set minus
/// one voter (the differencing move at the source). If this set and the honest
/// set were ever *both* decrypted, the multiset difference would disclose the
/// dropped voter's ballot.
const EQUIVOCATED_VOTERS: [u8; 1] = [0];

/// How a dishonest mixer subverts its shuffle (model config, [`SymbolicModel`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DishonestKind {
    /// A real shuffle, but with a permutation the adversary knows: the multiset
    /// is preserved (so it verifies and an honest signer accepts it), but it
    /// adds no opaque layer. Attacks privacy/linkage.
    KnownPermutation,
    /// A forged shuffle: the output multiset differs from the input (here, a
    /// voter is dropped). An honest verifier rejects it. Attacks integrity.
    Forge,
    /// Mixes honestly, but SKIPS the input-ballot anchor check — an operator
    /// that processes an illegitimate ballot set. Attacks privacy via split
    /// views: a whole illegitimate-strand quorum of these lets a second,
    /// divergent strand complete.
    SkipsAnchor,
}

/// Encode a ballots token body: the voter multiset plus a salt (so an
/// equivocating manager's successive ballots stay distinct artifacts).
fn encode_ballots(voters: &[u8], salt: usize) -> Vec<u8> {
    serde_json::to_vec(&(voters, salt)).expect("encode ballots token")
}

/// The voter multiset carried by a ballots token body, or `None` if the body
/// is not a ballots token. The stored body is `Vec::<u8>::ser(token)` (the wire
/// constructor frames it), so the framing is undone before the JSON is parsed.
fn decode_ballots(body: &[u8]) -> Option<Vec<u8>> {
    let token = <Vec<u8>>::deser(body).ok()?;
    serde_json::from_slice::<(Vec<u8>, usize)>(&token)
        .ok()
        .map(|(voters, _salt)| voters)
}

/// Encode a mix token body: `(mixer, input)` keeps distinct mixes' outputs
/// distinct (as distinct real shuffles would be), and `(output_voters, opaque)`
/// is the semantic payload — the multiset this mix claims to output and whether
/// it added an opaque layer.
fn encode_mix(
    mixer: TrusteeIndex,
    input: &CiphertextsHash,
    output_voters: &[u8],
    opaque: bool,
) -> Vec<u8> {
    serde_json::to_vec(&(mixer, format!("{input:?}"), output_voters, opaque))
        .expect("encode mix token")
}

/// The `(output multiset, opaque)` carried by a mix token body, or `None` if
/// the body is not a mix token. The stored body is `Vec::<u8>::ser(token)`, so
/// the framing is undone before the JSON is parsed.
fn decode_mix(body: &[u8]) -> Option<(Vec<u8>, bool)> {
    let token = <Vec<u8>>::deser(body).ok()?;
    let (_mixer, _input, output_voters, opaque): (TrusteeIndex, String, Vec<u8>, bool) =
        serde_json::from_slice(&token).ok()?;
    Some((output_voters, opaque))
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
    /// The dishonest mixers, by 1-based index: how each subverts its shuffle.
    /// A dishonest trustee is honest everywhere else (this scopes dishonesty to
    /// the mixing phase, which is what the privacy and integrity properties are
    /// about). Fixed for the run, not budgeted. The interesting cases keep this
    /// sub-threshold; a threshold-filling set is used only in negative-control
    /// tests, to show a property bites.
    dishonest_mixers: HashMap<TrusteeIndex, DishonestKind>,
    /// Whether honest trustees verify a mix before signing it (the integrity
    /// defense). Always true except in the negative control that shows the
    /// integrity property fails when the defense is removed.
    honest_verification: bool,
    /// The mixing quorum an equivocated ballots names. `None` = the same list
    /// as the honest ballots. A DISJOINT quorum (needs n >= 2*threshold) is
    /// what makes the type-2 split-view attack constructible: the two strands
    /// are mixed and decrypted by disjoint trustee sets, so no trustee ever
    /// crosses strands.
    equivocation_quorum: Option<Vec<TrusteeIndex>>,
    /// A FIXED split view (adversarial b4): each trustee sees only the strands
    /// whose mixing quorum includes it — the coherent partition a split-view b4
    /// presents. Deterministic (no branching), unlike the `Withhold` search, so
    /// it stays tractable at the n it takes to make type-2 reachable
    /// (n >= 2*threshold). It asks the outcome directly: given the partition,
    /// does the search reach two differently-decrypted strands?
    fixed_split_view: bool,
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
            share_enc_keys.push(KeyPair::<C>::generate().pkey.y);
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
            dishonest_mixers: HashMap::new(),
            honest_verification: true,
            equivocation_quorum: None,
            fixed_split_view: false,
            manager,
            trustees,
            configuration,
            configuration_hash,
            mixing_trustees: (1..=threshold).collect(),
            memo: Mutex::new(HashMap::new()),
        }
    }

    /// Trustee `index` (1-based) mixes dishonestly in the given way. Chainable
    /// (a negative control makes every mixer dishonest).
    fn with_dishonest_mixer(mut self, index: TrusteeIndex, kind: DishonestKind) -> Self {
        self.dishonest_mixers.insert(index, kind);
        self
    }

    /// Trustee `index` mixes with a permutation the adversary knows — a valid
    /// shuffle that adds no privacy.
    fn with_known_permutation_mixer(self, index: TrusteeIndex) -> Self {
        self.with_dishonest_mixer(index, DishonestKind::KnownPermutation)
    }

    /// Trustee `index` forges its mix — drops a voter. Honest signers must
    /// reject it, so it never reaches decryption.
    fn with_forging_mixer(self, index: TrusteeIndex) -> Self {
        self.with_dishonest_mixer(index, DishonestKind::Forge)
    }

    /// Remove the honest-verification defense (negative control only).
    fn without_honest_verification(mut self) -> Self {
        self.honest_verification = false;
        self
    }

    /// Allow up to `budget` [`Turn::DropCommit`] cycles in the exploration.
    fn with_dropped_commit_budget(mut self, budget: usize) -> Self {
        self.budgets.dropped_commits = budget;
        self
    }

    /// Allow up to `budget` [`Turn::CrashBeforeRecord`] cycles in the
    /// exploration.
    fn with_crash_before_record_budget(mut self, budget: usize) -> Self {
        self.budgets.crashes_before_record = budget;
        self
    }

    /// Allow up to `budget` [`Turn::EquivocateBallots`] posts in the
    /// exploration.
    fn with_ballots_equivocation_budget(mut self, budget: usize) -> Self {
        self.budgets.ballots_equivocations = budget;
        self
    }

    /// Allow up to `budget` [`Turn::Withhold`] events in the exploration.
    /// Turns off the observation-timing compression (ruling 1).
    fn with_withholding_budget(mut self, budget: usize) -> Self {
        self.budgets.withholdings = budget;
        self
    }

    /// The equivocated ballots names `quorum` as its mixing list (a disjoint
    /// quorum enables the type-2 split-view attack).
    fn with_equivocation_quorum(mut self, quorum: Vec<TrusteeIndex>) -> Self {
        self.equivocation_quorum = Some(quorum);
        self
    }

    /// b4 presents each trustee only the strands whose mixing quorum includes
    /// it (a coherent, fixed split view).
    fn with_fixed_split_view(mut self) -> Self {
        self.fixed_split_view = true;
        self
    }

    /// The mixing quorum of each strand root on the board (from the ballots'
    /// `trustees` field): which trustees are meant to process it.
    fn strand_quorums(&self, state: &SystemState) -> HashMap<CiphertextsHash, Vec<TrusteeIndex>> {
        let mut q = HashMap::new();
        for bytes in &state.board {
            if let Ok(message) = ProtocolMessage::<C>::deser(bytes) {
                if let Ok((Predicate::Ballots(b), _)) = verify(&message, &self.configuration) {
                    q.insert(b.ciphertexts, b.trustees.clone());
                }
            }
        }
        q
    }

    /// The board as trustee `i` sees it: the true board minus every row that
    /// belongs to a strand b4 withholds from `i`. Fault-free, `withheld[i]` is
    /// empty and this is the whole board. A row with no strand (the shared DKG
    /// artifacts: Configuration, Shares, PublicKey) is always visible.
    fn visible_board(&self, state: &SystemState, i: usize) -> Vec<ProtocolMessage<C>> {
        let hidden = &state.withheld[i];
        if hidden.is_empty() && !self.fixed_split_view {
            return state
                .board
                .iter()
                .map(|bytes| ProtocolMessage::<C>::deser(bytes).expect("well-formed message"))
                .collect();
        }
        let roots = self.board_strand_roots(state);
        // Under a fixed split view, `i` sees a strand only if its mixing quorum
        // includes trustee `i+1` (1-based). Together with any withheld strands.
        let quorums = self.fixed_split_view.then(|| self.strand_quorums(state));
        let trustee = (i + 1) as TrusteeIndex;
        state
            .board
            .iter()
            .map(|bytes| ProtocolMessage::<C>::deser(bytes).expect("well-formed message"))
            .filter(|message| match self.message_strand(message, &roots) {
                Some(root) => {
                    if hidden.contains(&strand_key(&root)) {
                        return false;
                    }
                    if let Some(quorums) = &quorums {
                        if let Some(q) = quorums.get(&root) {
                            return q.contains(&trustee);
                        }
                    }
                    true
                }
                None => true,
            })
            .collect()
    }

    /// The ballots-root hash (hex) of every strand currently on the board — the
    /// withholdable units. Distinct ballots messages are distinct strands.
    fn ballots_roots(&self, state: &SystemState) -> Vec<String> {
        let mut roots = Vec::new();
        for bytes in &state.board {
            if let Ok(message) = ProtocolMessage::<C>::deser(bytes) {
                if let Ok((Predicate::Ballots(b), _)) = verify(&message, &self.configuration) {
                    let key = strand_key(&b.ciphertexts);
                    if !roots.contains(&key) {
                        roots.push(key);
                    }
                }
            }
        }
        roots
    }

    /// Map every ciphertexts hash on the board (a ballots root or a mix output)
    /// to its strand root, by following mix inputs back to the ballots.
    fn board_strand_roots(&self, state: &SystemState) -> HashMap<CiphertextsHash, CiphertextsHash> {
        // Ballots roots (each is its own root) and mix output→input edges.
        let mut is_root: Vec<CiphertextsHash> = Vec::new();
        let mut edge: HashMap<CiphertextsHash, CiphertextsHash> = HashMap::new();
        for bytes in &state.board {
            let Ok(message) = ProtocolMessage::<C>::deser(bytes) else {
                continue;
            };
            match verify(&message, &self.configuration) {
                Ok((Predicate::Ballots(b), _)) => is_root.push(b.ciphertexts),
                Ok((Predicate::Mix(m), _)) => {
                    edge.insert(m.output, m.input);
                }
                _ => {}
            }
        }
        let mut roots: HashMap<CiphertextsHash, CiphertextsHash> = HashMap::new();
        for r in &is_root {
            roots.insert(*r, *r);
        }
        let bound = state.board.len() + 1;
        for &output in edge.keys() {
            // Walk output → input → … until a ballots root (or give up).
            let mut cursor = output;
            let mut steps = 0;
            let root = loop {
                if is_root.contains(&cursor) {
                    break Some(cursor);
                }
                match edge.get(&cursor) {
                    Some(&input) => cursor = input,
                    None => break None,
                }
                steps += 1;
                if steps > bound {
                    break None;
                }
            };
            if let Some(root) = root {
                roots.insert(output, root);
            }
        }
        roots
    }

    /// The strand root a message belongs to, or `None` if it belongs to no
    /// strand (the shared DKG artifacts) or its chain cannot be resolved.
    fn message_strand(
        &self,
        message: &ProtocolMessage<C>,
        roots: &HashMap<CiphertextsHash, CiphertextsHash>,
    ) -> Option<CiphertextsHash> {
        let (predicate, _) = verify(message, &self.configuration).ok()?;
        let ciphertexts = match predicate {
            Predicate::Ballots(b) => b.ciphertexts,
            Predicate::Mix(m) => m.output,
            Predicate::MixSignature(m) => m.output,
            Predicate::PartialDecryptions(p) => p.ciphertexts,
            Predicate::Plaintexts(p) => p.ciphertexts,
            // Shares, PublicKey, ConfigurationValid: no strand.
            _ => return None,
        };
        roots.get(&ciphertexts).copied()
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
            fail_fetch: false,
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
            outgoing.extend(self.execute_symbolic(i, action, view));
        }
        Ok(outgoing)
    }

    /// Whether trustee `i` enforces the input-ballot anchor: an honest actor
    /// (not dishonest in any way, defense enabled) does; a dishonest operator
    /// (`dishonest_mixers`) or the disabled-defense negative control does not.
    fn checks_anchor(&self, i: &TrusteeIndex) -> bool {
        self.honest_verification && !self.dishonest_mixers.contains_key(i)
    }

    /// Whether the ballots at `input` (a first mix's input) are the anchor —
    /// the legitimate ballot set. `false` if the ballots are illegitimate or
    /// unresolvable.
    fn anchor_ok(&self, input: &CiphertextsHash, view: &MessageStore<C>) -> bool {
        let (ballots, _) = view_content_maps(view);
        let mut anchor = HONEST_VOTERS.to_vec();
        anchor.sort_unstable();
        ballots
            .get(input)
            .map(|voters| {
                let mut v = voters.clone();
                v.sort_unstable();
                v == anchor
            })
            .unwrap_or(false)
    }

    /// Execute one derived action symbolically: assemble the real wire message
    /// around a token body. `view` is the trustee's board, needed to read the
    /// content its mixes shuffle and its signatures verify.
    ///
    /// Token discipline: a token is a deterministic function of the action's
    /// hash-bound inputs; the producer's index is included exactly when the
    /// real artifact would differ per trustee (shares, mixes, partial
    /// decryptions carry per-trustee randomness or key shares) and omitted
    /// when the real computation must agree across trustees (the joint public
    /// key, the combined plaintexts) — otherwise hash-equality agreement
    /// between trustees would spuriously fail.
    fn execute_symbolic(
        &self,
        i: usize,
        action: &Action,
        view: &MessageStore<C>,
    ) -> Vec<ProtocolMessage<C>> {
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
            Action::ComputeMix(cfg, pk, source, input, self_index) => {
                // Anchor check (§ input-ballot legitimacy): the FIRST mixer
                // engages the ballots here — its compute IS its signature (the
                // datalog derives `mix_signature` from `mix`), so this is the
                // one place it can enforce the anchor. An honest first mixer
                // refuses to mix a ballot set that is not the anchor; a
                // dishonest one processes it anyway.
                if *source == MixSource::Ballots
                    && self.checks_anchor(self_index)
                    && !self.anchor_ok(input, view)
                {
                    return vec![];
                }
                let (ballots, mixes) = view_content_maps(view);
                let bound = ballots.len() + mixes.len() + 1;
                let (input_voters, _) = content_at(input, &ballots, &mixes, bound)
                    .expect("a mix's input has resolvable content");
                // Honest by default; a dishonest mixer subverts its own shuffle.
                // `SkipsAnchor` still mixes honestly — its dishonesty is only
                // the missing anchor check above.
                let (output_voters, opaque) = match self.dishonest_mixers.get(self_index) {
                    Some(DishonestKind::Forge) => {
                        // Drop a voter: the forged output is a strict subset.
                        let mut v = input_voters.clone();
                        v.pop();
                        (v, false)
                    }
                    // A valid shuffle (multiset preserved), but not opaque.
                    Some(DishonestKind::KnownPermutation) => (input_voters.clone(), false),
                    Some(DishonestKind::SkipsAnchor) | None => (input_voters.clone(), true),
                };
                let token = encode_mix(*self_index, input, &output_voters, opaque);
                vec![ProtocolMessage::<C>::mix(t, DATE, *cfg, *pk, *input, &token)]
            }
            Action::SignMix(cfg, pk, source, input, output, self_index) => {
                let signer = ProtocolMessage::<C>::mix_signature(t, DATE, *cfg, *pk, *input, *output);
                // A mixing-dishonest trustee (or the disabled-defense negative
                // control) signs without verifying anything.
                let blind = !self.honest_verification
                    || matches!(
                        self.dishonest_mixers.get(self_index),
                        Some(DishonestKind::KnownPermutation) | Some(DishonestKind::Forge)
                    );
                if blind {
                    return vec![signer];
                }
                // Anchor check: signing the FIRST mix is where every other
                // quorum member engages the ballots. An honest signer refuses a
                // first mix rooted at a ballot set that is not the anchor; a
                // `SkipsAnchor` operator does not. Together with the first
                // mixer's check, an illegitimate strand needs its WHOLE quorum
                // dishonest to gather threshold signatures.
                if *source == MixSource::Ballots
                    && self.checks_anchor(self_index)
                    && !self.anchor_ok(input, view)
                {
                    return vec![];
                }
                // Shuffle-proof verification: the mix's output multiset must
                // equal its input's. A forged mix fails, so it never gathers
                // the threshold signatures it needs to extend the chain.
                let (ballots, mixes) = view_content_maps(view);
                let bound = ballots.len() + mixes.len() + 1;
                let out_ms = content_at(output, &ballots, &mixes, bound).map(|(m, _)| m);
                let in_ms = content_at(input, &ballots, &mixes, bound).map(|(m, _)| m);
                match (out_ms, in_ms) {
                    (Some(o), Some(i)) if o == i => vec![signer],
                    _ => vec![],
                }
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
    /// fault-configured) transport and persistence, its effects merged into
    /// `next`. `None` means the cycle was idle — not a transition (see the
    /// guard notes). `Some(crashed)` reports whether an injected crash
    /// actually fired (its budget is only spent when it did — a
    /// crash-before-record on a cycle that never writes a record, e.g. a pure
    /// re-send, degenerates to the honest cycle and folds with it).
    fn trustee_cycle(
        &self,
        state: &SystemState,
        next: &mut SystemState,
        i: usize,
        fault: CycleFault,
    ) -> Option<bool> {
        let persistence = self.persistence_from(&state.trustees[i]);
        let client_persistence = CrashingPersistence {
            inner: persistence.clone(),
            fail_record: fault == CycleFault::CrashBeforeRecord,
        };
        let (transport, staged, committed) = self.transport_for(
            state,
            self.visible_board(state, i),
            fault == CycleFault::DropCommits,
        );
        let outcome = block_on(async {
            let mut client = BoardClient::connect(transport, client_persistence).await?;
            client.update().await?;
            let produced = self.symbolic_step(i, client.view())?;
            let produced_any = !produced.is_empty();
            if produced_any {
                client.post(produced).await?;
            }
            Ok::<bool, anyhow::Error>(produced_any)
        });

        // Ruling 1: with withholding enabled, an observe-only cycle that grew
        // the committed set is a real transition (its pins decide whether a
        // later gate fires), so the observation-timing compression is off and
        // the `next == *state` guard in `successor` prunes genuinely-idle
        // cycles instead. Otherwise (fault-free/benign) the compression stands.
        let compress = self.budgets.withholdings == 0;
        let mut crashed = false;
        match outcome {
            Ok(produced_any) => {
                if compress && !produced_any {
                    return None;
                }
            }
            // An injected crash is a death, not a datalog halt: keep the
            // durable state written so far and move on — a restarted trustee
            // recomputes next cycle (§6.4: nothing was recorded, so nothing
            // pins the slot).
            Err(e) if format!("{e:#}").contains(CRASH_SENTINEL) => crashed = true,
            Err(e) => {
                let msg = format!("{e:#}");
                let reason = if msg.contains("anti-rewrite") {
                    HaltReason::CompletenessGate
                } else if msg.contains("colliding") {
                    HaltReason::Collision
                } else {
                    HaltReason::Other
                };
                next.halted[i] = Some(reason);
            }
        }
        next.trustees[i] = Self::durable_from(&persistence);
        Self::merge_transport(next, staged, committed);
        Some(crashed)
    }

    fn successor(&self, state: &SystemState, turn: &Turn) -> Option<SystemState> {
        let mut next = state.clone();

        match turn {
            Turn::Trustee(i) => {
                self.trustee_cycle(state, &mut next, *i, CycleFault::None)?;
            }
            Turn::DropCommit(i) => {
                self.trustee_cycle(state, &mut next, *i, CycleFault::DropCommits)?;
                next.faults.dropped_commits += 1;
            }
            Turn::CrashBeforeRecord(i) => {
                let crashed =
                    self.trustee_cycle(state, &mut next, *i, CycleFault::CrashBeforeRecord)?;
                if crashed {
                    next.faults.crashes_before_record += 1;
                }
            }
            Turn::Withhold(i, root) => {
                // b4 begins hiding strand `root` from trustee `i`. No cycle
                // runs; the effect is on what `i` sees next.
                if state.withheld[*i].contains(root) {
                    return None;
                }
                next.withheld[*i].push(root.clone());
                next.faults.withholdings += 1;
            }
            Turn::PostBallots => {
                let pk_hash = self.public_key_hash_on(state)?;
                let token = encode_ballots(&HONEST_VOTERS, 0);
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
            Turn::EquivocateBallots => {
                // Same heads as the honest ballots (the differencing attack
                // holds everything equal except the ballot set); a different
                // voter multiset, salted by the counter so successive
                // equivocations are distinct artifacts.
                let pk_hash = self.public_key_hash_on(state)?;
                let k = state.faults.ballots_equivocations;
                let token = encode_ballots(&EQUIVOCATED_VOTERS, k + 1);
                let quorum = self
                    .equivocation_quorum
                    .clone()
                    .unwrap_or_else(|| self.mixing_trustees.clone());
                let message = ProtocolMessage::<C>::ballots(
                    &self.manager,
                    DATE,
                    self.configuration_hash,
                    pk_hash,
                    quorum,
                    &token,
                );
                let (transport, staged, committed) = self.transport_for(state, Vec::new(), false);
                block_on(transport.publish(&message)).expect("model publish cannot fail");
                Self::merge_transport(&mut next, staged, committed);
                next.faults.ballots_equivocations += 1;
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
            halted: vec![None; self.n],
        }]
    }

    /// The lookahead: only turns that actually move the system are offered.
    fn actions(&self, state: &Self::State, actions: &mut Vec<Self::Action>) {
        // A HALTED trustee takes no further turns; the others keep running
        // (halting is per-trustee, see `SystemState::halted`).
        let active: Vec<usize> = (0..self.n).filter(|i| state.halted[*i].is_none()).collect();
        let mut candidates: Vec<Turn> = active.iter().copied().map(Turn::Trustee).collect();
        // Fault turns, while their budget lasts. The lookahead prunes the
        // pointless ones for free: a faulty cycle of an idle trustee produces
        // nothing and is not a transition.
        if state.faults.dropped_commits < self.budgets.dropped_commits {
            candidates.extend(active.iter().copied().map(Turn::DropCommit));
        }
        if state.faults.crashes_before_record < self.budgets.crashes_before_record {
            candidates.extend(active.iter().copied().map(Turn::CrashBeforeRecord));
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
        // The adversarial manager can post a divergent second ballots while
        // its budget lasts (meaningful only once a first ballots exists).
        if has_ballots && state.faults.ballots_equivocations < self.budgets.ballots_equivocations
        {
            candidates.push(Turn::EquivocateBallots);
        }
        // Adversarial b4 can begin withholding a whole strand (ballots + its
        // lineage) from any still-active trustee, while its budget lasts.
        if state.faults.withholdings < self.budgets.withholdings {
            let strands = self.ballots_roots(state);
            for &i in &active {
                for root in &strands {
                    if !state.withheld[i].contains(root) {
                        candidates.push(Turn::Withhold(i, root.clone()));
                    }
                }
            }
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
        let mut props = Vec::new();

        // === Functional properties: the assets themselves, UNCONDITIONAL over
        // the entire fault space (the no-exemption pattern). They read the
        // symbolic content, not protocol mechanisms — a violation is a real
        // privacy or integrity failure however it was reached.

        // PRIVACY (differencing): the adversary must never obtain two
        // *different* decrypted ballot sets. Two threshold-decrypted sets with
        // different multisets is the differencing attack consummated —
        // regardless of how the divergence arose.
        props.push(Property::<Self>::always(
            "all decrypted sets carry the same ballots",
            |model, state| {
                let board = BoardContents::read(model, state);
                let bound = state.board.len() + 1;
                let mut seen: Option<Vec<u8>> = None;
                for set in board.decrypted_sets(model.mixing_trustees.len()) {
                    let Some((voters, _)) = board.content_at(&set, bound) else {
                        return false; // a decrypted set with no valid lineage
                    };
                    match &seen {
                        None => seen = Some(voters),
                        Some(first) => {
                            if *first != voters {
                                return false;
                            }
                        }
                    }
                }
                true
            },
        ));

        // PRIVACY (linkage): every decrypted set must have passed through at
        // least one opaque (honest, adversary-unknown) shuffle. A set decrypted
        // with no opaque layer means the adversary knows the whole permutation
        // and can re-link inputs to outputs.
        props.push(Property::<Self>::always(
            "every decrypted set passed through an honest shuffle",
            |model, state| {
                let board = BoardContents::read(model, state);
                let bound = state.board.len() + 1;
                board.decrypted_sets(model.mixing_trustees.len()).iter().all(|set| {
                    matches!(board.content_at(set, bound), Some((_, opaque)) if opaque)
                })
            },
        ));

        // INTEGRITY: any published plaintexts must carry exactly the honest
        // ballot set. A completed output whose multiset differs from the
        // ballots means a corrupted mix reached decryption.
        props.push(Property::<Self>::always(
            "published plaintexts match the honest ballots",
            |model, state| {
                let board = BoardContents::read(model, state);
                let bound = state.board.len() + 1;
                let mut honest = HONEST_VOTERS.to_vec();
                honest.sort_unstable();
                board.plaintext_sets.iter().all(|set| {
                    matches!(board.content_at(set, bound), Some((voters, _)) if voters == honest)
                })
            },
        ));

        let adversarial = self.budgets.ballots_equivocations > 0
            || self.budgets.withholdings > 0
            || self.fixed_split_view
            || !self.dishonest_mixers.is_empty();

        if !adversarial {
            // Safety, UNCONDITIONAL over the benign-fault space: no pattern
            // of at most budget-many benign faults may ever halt a trustee.
            props.push(Property::<Self>::always("no trustee halts", |_, state| {
                state.halted.iter().all(|h| h.is_none())
            }));
            // Liveness, in its strong form: on EVERY path the protocol
            // completes. Sound here, unlike in the crypto harness, because
            // the exploration is exhaustive (deterministic edges + dedup) and
            // acyclic (the board only grows), with no depth cap. The k-fault-
            // tolerance claim: the mailbox recovers every benign fault.
            //
            // Completion is plaintexts from every MIXING trustee, not every
            // trustee: the post-DKG quorum is the mixing list (of size ==
            // threshold) — both `ComputePartialDecryptions` and
            // `ComputePlaintexts` require `mixing_position` (decrypt.rs).
            // Non-mixing trustees go quiet after the DKG by design.
            props.push(Property::<Self>::eventually(
                "protocol completes",
                |model, state| plaintexts_on(state) == model.mixing_trustees.len(),
            ));
        } else {
            // Conditioned safety: only an adversary may cause a halt (benign
            // faults never do).
            props.push(Property::<Self>::always(
                "no trustee halts unless an adversary acted",
                |model, state| {
                    state.halted.iter().all(|h| h.is_none()) || adversarial_acted(model, state)
                },
            ));
            // Conditioned liveness: every path either completes or an adversary
            // acted. Weaker than the benign completion claim on purpose — an
            // adversary can always deny liveness, so completion is not promised
            // once one has acted. The functional safety properties above are
            // what hold regardless.
            props.push(Property::<Self>::eventually(
                "completes, or an adversary acted",
                |model, state| {
                    plaintexts_on(state) == model.mixing_trustees.len()
                        || adversarial_acted(model, state)
                },
            ));
            // Non-vacuity: the adversary actually acts on some path (else every
            // conditioned property passes without testing the attack).
            props.push(Property::<Self>::sometimes("an adversary acts", |model, state| {
                adversarial_acted(model, state)
            }));
        }
        // The halt-on-equivocation guard applies only on a consistent board,
        // where a trustee can see both ballots and halt on the collision. Under
        // a split view no trustee ever sees both — the anchor check, not a
        // halt, is the defense — so the guard is not expected there.
        if self.budgets.ballots_equivocations > 0 && !self.fixed_split_view {
            props.push(Property::<Self>::sometimes(
                "a trustee halts on the equivocation",
                |_, state| {
                    state.faults.ballots_equivocations > 0
                        && state.halted.iter().any(|h| h.is_some())
                },
            ));
        }
        if !self.dishonest_mixers.is_empty() {
            props.push(Property::<Self>::sometimes(
                "the dishonest mixer mixes",
                dishonest_mix_present,
            ));
        }
        if self.budgets.withholdings > 0 {
            props.push(Property::<Self>::sometimes(
                "b4 withholds a row from a trustee",
                |_, state| state.faults.withholdings > 0,
            ));
            props.push(Property::<Self>::sometimes(
                "the completeness gate halts a trustee under withholding",
                |_, state| {
                    state.faults.withholdings > 0
                        && state.halted.contains(&Some(HaltReason::CompletenessGate))
                },
            ));
        }
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
        if self.budgets.crashes_before_record > 0 {
            props.push(Property::<Self>::sometimes(
                "a crash before the own-post record fires",
                |_, state| state.faults.crashes_before_record > 0,
            ));
            props.push(Property::<Self>::sometimes(
                "the protocol completes despite a crash before the record",
                |model, state| {
                    state.faults.crashes_before_record > 0
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

/// Whether the configured dishonest mixer has posted a mix — the adversary has
/// acted. (It only ever mixes dishonestly, so any of its mixes counts.)
fn dishonest_mix_present(model: &SymbolicModel, state: &SystemState) -> bool {
    if model.dishonest_mixers.is_empty() {
        return false;
    }
    state.board.iter().any(|bytes| {
        ProtocolMessage::<C>::deser(bytes)
            .ok()
            .and_then(|m| verify(&m, &model.configuration).ok())
            .map(|(p, _)| matches!(p, Predicate::Mix(m) if model.dishonest_mixers.contains_key(&m.sender)))
            .unwrap_or(false)
    })
}

/// Whether any modeled adversary has acted on this path: the manager
/// equivocated, or the dishonest mixer mixed. The excuse the conditioned
/// liveness and safety properties allow.
fn adversarial_acted(model: &SymbolicModel, state: &SystemState) -> bool {
    (model.budgets.ballots_equivocations > 0 && state.faults.ballots_equivocations > 0)
        || (model.budgets.withholdings > 0 && state.faults.withholdings > 0)
        || (model.fixed_split_view && state.board.len() > 1)
        || dishonest_mix_present(model, state)
}

/// A mix chain read from ballots/mix bodies: `output hash → (input hash, output
/// multiset, opaque)`. Shared by the board-side ([`BoardContents`]) and the
/// view-side ([`view_content_maps`]) readers.
type MixMap = HashMap<CiphertextsHash, (CiphertextsHash, Vec<u8>, bool)>;

/// The symbolic content at a ciphertexts hash: the sorted voter multiset it
/// carries, and whether its mix chain includes an opaque (honest) layer.
/// Computed by walking the chain from the hash back to its ballots root —
/// `None` if the chain is dangling or malformed.
fn content_at(
    hash: &CiphertextsHash,
    ballots: &HashMap<CiphertextsHash, Vec<u8>>,
    mixes: &MixMap,
    depth: usize,
) -> Option<(Vec<u8>, bool)> {
    if depth == 0 {
        return None; // chain longer than the board: malformed
    }
    if let Some(voters) = ballots.get(hash) {
        let mut sorted = voters.clone();
        sorted.sort_unstable();
        return Some((sorted, false));
    }
    let (input, output_voters, opaque) = mixes.get(hash)?;
    // The multiset is what this mix claims to output; opacity accumulates along
    // the chain (any honest layer makes the whole chain opaque).
    let (_, prev_opaque) = content_at(input, ballots, mixes, depth - 1)?;
    let mut sorted = output_voters.clone();
    sorted.sort_unstable();
    Some((sorted, prev_opaque || *opaque))
}

/// The ballots and mix maps a trustee reads from its own view (`MessageStore`),
/// to compute the content its mixes shuffle and its signatures verify.
fn view_content_maps(view: &MessageStore<C>) -> (HashMap<CiphertextsHash, Vec<u8>>, MixMap) {
    let mut ballots: HashMap<CiphertextsHash, Vec<u8>> = HashMap::new();
    let mut mixes: MixMap = HashMap::new();
    for predicate in view.get_predicates() {
        match predicate {
            Predicate::Ballots(b) => {
                if let Some(voters) = view.ballots_body(&b.ciphertexts).and_then(decode_ballots) {
                    ballots.insert(b.ciphertexts, voters);
                }
            }
            Predicate::Mix(m) => {
                if let Some((out, opaque)) =
                    view.mix_body_by_output(&m.output).and_then(decode_mix)
                {
                    mixes.insert(m.output, (m.input, out, opaque));
                }
            }
            _ => {}
        }
    }
    (ballots, mixes)
}

/// The board read symbolically: ballot sets, mix chain, decryption counts, and
/// published-plaintext sets. Built by one pass over the board through the real
/// `verify`; the privacy and integrity properties reason over it.
struct BoardContents {
    /// ciphertexts hash → voter multiset (ballots roots).
    ballots: HashMap<CiphertextsHash, Vec<u8>>,
    /// mix output hash → (input hash, output multiset, opaque).
    mixes: MixMap,
    /// ciphertexts hash → how many partial decryptions it has.
    pdec_counts: HashMap<CiphertextsHash, usize>,
    /// ciphertexts hashes that have a published `Plaintexts`.
    plaintext_sets: Vec<CiphertextsHash>,
}

impl BoardContents {
    fn read(model: &SymbolicModel, state: &SystemState) -> Self {
        let mut c = BoardContents {
            ballots: HashMap::new(),
            mixes: HashMap::new(),
            pdec_counts: HashMap::new(),
            plaintext_sets: Vec::new(),
        };
        for bytes in &state.board {
            let Ok(message) = ProtocolMessage::<C>::deser(bytes) else {
                continue;
            };
            if message.message_type == MessageType::Configuration {
                continue;
            }
            let Ok((predicate, body)) = verify(&message, &model.configuration) else {
                continue;
            };
            match predicate {
                Predicate::Ballots(b) => {
                    if let Some(voters) = body.as_deref().and_then(decode_ballots) {
                        c.ballots.insert(b.ciphertexts, voters);
                    }
                }
                Predicate::Mix(m) => {
                    if let Some((out, opaque)) = body.as_deref().and_then(decode_mix) {
                        c.mixes.insert(m.output, (m.input, out, opaque));
                    }
                }
                Predicate::PartialDecryptions(p) => {
                    *c.pdec_counts.entry(p.ciphertexts).or_insert(0) += 1;
                }
                Predicate::Plaintexts(p) => c.plaintext_sets.push(p.ciphertexts),
                _ => {}
            }
        }
        c
    }

    /// The content at `hash`, walking the mix chain to its ballots root.
    fn content_at(&self, hash: &CiphertextsHash, bound: usize) -> Option<(Vec<u8>, bool)> {
        content_at(hash, &self.ballots, &self.mixes, bound)
    }

    /// Sets with at least `threshold` partial decryptions — enough to
    /// reconstruct the plaintexts, so the disclosure has happened.
    fn decrypted_sets(&self, threshold: usize) -> Vec<CiphertextsHash> {
        self.pdec_counts
            .iter()
            .filter(|(_, count)| **count >= threshold)
            .map(|(hash, _)| *hash)
            .collect()
    }
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

/// Run the checker and assert the named property was VIOLATED (a counterexample
/// was found). Negative controls: they prove a property has teeth by removing
/// the defense it checks and confirming it then fails.
fn expect_violation(model: SymbolicModel, property: &str) {
    let checker = model.checker().threads(1).spawn_bfs().join();
    assert!(
        checker.discoveries().iter().any(|(name, _)| *name == property),
        "expected a counterexample for `{property}`, but it held"
    );
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

/// Adversarial b4 withholds board rows (rewrite / split view). At n=2/t=2 the
/// per-sender quorum arithmetic makes type-2 split-view differencing
/// unconstructible (2·threshold > n), so privacy holds; the §6.3 gate + §6.4
/// own-post record defend the type-1 rewrite case (a trustee fed a board
/// missing something it pinned halts before re-deriving a divergent artifact).
#[test]
fn model_check_symbolic_two_trustees_withholding() {
    check(
        SymbolicModel::new(2, 2).with_withholding_budget(1),
        "n=2 t=2 withhold<=1",
    );
}

/// Withholding combined with equivocation — the type-1 differencing attempt
/// (two ballots, b4 rewrites which one a trustee sees). Privacy must hold.
#[test]
fn model_check_symbolic_two_trustees_equivocation_and_withholding() {
    check(
        SymbolicModel::new(2, 2)
            .with_ballots_equivocation_budget(1)
            .with_withholding_budget(1),
        "n=2 t=2 equiv<=1 withhold<=1",
    );
}

/// A sub-threshold dishonest mixer that shuffles with a permutation the
/// adversary knows: a valid shuffle (it completes), but the privacy guarantee
/// must still hold because the other, honest mixer contributes an opaque layer.
#[test]
fn model_check_symbolic_two_trustees_known_permutation_mixer() {
    check(
        SymbolicModel::new(2, 2).with_known_permutation_mixer(1),
        "n=2 t=2 known-perm mixer t1",
    );
}

/// A sub-threshold dishonest mixer that forges its shuffle (drops a voter).
/// Honest verification must reject it, so the forged content never reaches
/// decryption: integrity holds, and the protocol does not complete.
#[test]
fn model_check_symbolic_two_trustees_forging_mixer() {
    check(
        SymbolicModel::new(2, 2).with_forging_mixer(1),
        "n=2 t=2 forging mixer t1",
    );
}

/// The forging mixer at the second position: the honest first mixer shuffles,
/// then the forger corrupts, and the honest first mixer must refuse to sign the
/// forgery.
#[test]
fn model_check_symbolic_two_trustees_forging_mixer_pos2() {
    check(
        SymbolicModel::new(2, 2).with_forging_mixer(2),
        "n=2 t=2 forging mixer t2",
    );
}

/// The type-2 split view at n=4/t=2 (2*threshold == n, so two disjoint quorums
/// exist): an adversarial manager equivocates ballots naming a disjoint quorum
/// `{3,4}`, and b4 shows `{1,2}` only the legitimate strand and `{3,4}` only
/// the illegitimate one. With the anchor check in force, honest `{3,4}` refuse
/// to process the illegitimate first mix, so its strand never completes — only
/// one set is decrypted and privacy holds. This is the defense the anchor
/// provides, threshold-robustly.
#[test]
fn model_check_symbolic_split_view_anchored() {
    check(
        SymbolicModel::new(4, 2)
            .with_ballots_equivocation_budget(1)
            .with_equivocation_quorum(vec![3, 4])
            .with_fixed_split_view(),
        "n=4 t=2 split-view (anchor enforced)",
    );
}

/// Negative control — the anchor has teeth: the whole illegitimate-strand
/// quorum `{3,4}` SKIPS the anchor check, so the illegitimate strand completes
/// alongside the legitimate one and the two decrypt to different ballot sets —
/// the differencing attack consummated. Confirms the anchor is load-bearing
/// (and that it takes a full dishonest quorum, i.e. threshold-many, to defeat).
#[test]
fn anchor_property_has_teeth() {
    expect_violation(
        SymbolicModel::new(4, 2)
            .with_ballots_equivocation_budget(1)
            .with_equivocation_quorum(vec![3, 4])
            .with_fixed_split_view()
            .with_dishonest_mixer(3, DishonestKind::SkipsAnchor)
            .with_dishonest_mixer(4, DishonestKind::SkipsAnchor),
        "all decrypted sets carry the same ballots",
    );
}

/// Negative control — integrity has teeth: remove honest verification, and a
/// forged mix reaches decryption, publishing plaintexts that DON'T match the
/// ballots. Confirms the integrity property is not passing vacuously in
/// [`model_check_symbolic_two_trustees_forging_mixer`] (where it holds only
/// because honest verification stalls the forgery).
#[test]
fn integrity_property_has_teeth() {
    expect_violation(
        SymbolicModel::new(2, 2)
            .with_forging_mixer(1)
            .without_honest_verification(),
        "published plaintexts match the honest ballots",
    );
}

/// Negative control — privacy/linkage has teeth: make BOTH mixers use
/// adversary-known permutations (a threshold-filling dishonest set), and the
/// decrypted set has no opaque layer, so linkage fails. Confirms the property
/// is not vacuous when a genuine honest mixer is present.
#[test]
fn linkage_property_has_teeth() {
    expect_violation(
        SymbolicModel::new(2, 2)
            .with_known_permutation_mixer(1)
            .with_known_permutation_mixer(2),
        "every decrypted set passed through an honest shuffle",
    );
}

/// The other side of the §6.4 seam: crashes after staging, before the record —
/// recovery must be recomputation (nothing pins the slot).
#[test]
fn model_check_symbolic_two_trustees_crashes() {
    check(
        SymbolicModel::new(2, 2).with_crash_before_record_budget(2),
        "n=2 t=2 crashes<=2",
    );
}

/// Fault interaction: both benign classes in one exploration — a crash on one
/// side of the commit point and a lost commit on the other, in every order and
/// interleaving.
#[test]
fn model_check_symbolic_two_trustees_mixed_faults() {
    check(
        SymbolicModel::new(2, 2)
            .with_dropped_commit_budget(1)
            .with_crash_before_record_budget(1),
        "n=2 t=2 drops<=1 crashes<=1",
    );
}

/// The first adversarial run: a manager who may post one divergent second
/// ballots message, at any reachable point, in every interleaving. On a
/// consistent board every trustee that acts on both ballots halts on the
/// collision, so privacy/differencing holds (unconditional), halts happen only
/// given the equivocation (conditioned safety), and every path either completes
/// or an adversary acted (conditioned liveness).
#[test]
fn model_check_symbolic_two_trustees_ballots_equivocation() {
    check(
        SymbolicModel::new(2, 2).with_ballots_equivocation_budget(1),
        "n=2 t=2 equivocations<=1",
    );
}

/// The stutter lemma, pinned (see "Deliberately NOT fault classes" in the
/// module docs): a benign fetch failure aborts the cycle with ZERO durable
/// footprint — no pins, no record, no stage, no commit — so it is
/// behaviorally identical to the trustee not being scheduled, and schedule
/// exploration already covers any finite number of them. This test is the
/// load-bearing assumption's tripwire: it fails if `BoardClient::update` ever
/// stops aborting atomically (e.g. starts admitting messages before the fetch
/// completes), at which point fetch failures stop being stutters and need
/// modeling.
#[test]
fn fetch_failure_has_no_durable_footprint() {
    let model = SymbolicModel::new(2, 2);
    let init = model.init_states().pop().expect("one init state");
    // Advance to a state with real content: trustee 1 posts its shares.
    let s1 = model
        .successor(&init, &Turn::Trustee(0))
        .expect("first cycle is productive");

    // Trustee 2 has work to do at s1 (it would admit t1's shares and post its
    // own) — run its cycle against a b4 that fails the fetch.
    let persistence = model.persistence_from(&s1.trustees[1]);
    let before = persistence.snapshot();
    let staged: Rc<RefCell<HashMap<String, Vec<u8>>>> =
        Rc::new(RefCell::new(s1.staged.iter().cloned().collect()));
    let committed: Rc<RefCell<Vec<ProtocolMessage<C>>>> = Rc::new(RefCell::new(Vec::new()));
    let transport = ModelTransport {
        view: model.visible_board(&s1, 1),
        staged: Rc::clone(&staged),
        committed: Rc::clone(&committed),
        drop_commits: false,
        fail_fetch: true,
    };

    let outcome = block_on(async {
        let mut client = BoardClient::connect(transport, persistence.clone()).await?;
        client.update().await?;
        Ok::<(), anyhow::Error>(())
    });

    assert!(
        outcome.is_err(),
        "a failed fetch must surface as an update error"
    );
    assert_eq!(
        persistence.snapshot(),
        before,
        "a failed fetch must not change durable state (no pins, no records)"
    );
    assert!(
        committed.borrow().is_empty(),
        "a failed fetch must not commit anything"
    );
    assert_eq!(
        staged.borrow().len(),
        s1.staged.len(),
        "a failed fetch must not stage anything"
    );
}
