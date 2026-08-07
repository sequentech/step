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
//! **Not yet bought — state folding.** The state holds the board as an
//! ordered log, and two interleavings differ in exactly that order, so
//! logically-equal states remain byte-distinct and fingerprint dedup never
//! fires: at n=2 this explores 153 states, the same tree as the crypto
//! harness (an exact structural cross-validation, and a measurement of zero
//! folding). Collapsing the tree into the graph requires quotienting state
//! identity by message order — sound only because the protocol layer is
//! order-insensitive (datalog consumes predicate *sets*) — a deliberate
//! modeling step not yet taken.
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

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Mutex;

use common::MemoryPersistence;
use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::elgamal::KeyPair;
use cryptography::utils::serialization::{VDeserializable, VSerializable};
use cryptography::utils::signatures::SignatureScheme;
use stateright::{Checker, Model, Property};

use braid::board::store::MessageStore;
use braid::board::transport::{MemoryBoard, MemoryTransport, StagedRef};
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

const TRUSTEES: usize = 2;
const THRESHOLD: usize = 2;
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Turn {
    /// Trustee `i` (0-based) runs one full update/infer/post cycle.
    Trustee(usize),
    /// The manager posts the ballots (a token), which it can only do once the
    /// DKG has published a public key.
    PostBallots,
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
    fn new() -> Self {
        let mut key_rng = C::get_rng();
        let manager = ProtocolManager::<C>::new(Sig::gen_signing_key(&mut key_rng));

        let mut signing_keys = Vec::with_capacity(TRUSTEES);
        let mut trustee_vks = Vec::with_capacity(TRUSTEES);
        let mut share_enc_keys = Vec::with_capacity(TRUSTEES);
        for _ in 0..TRUSTEES {
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
            THRESHOLD,
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
            manager,
            trustees,
            configuration,
            configuration_hash,
            mixing_trustees: (1..=THRESHOLD).collect(),
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
            Action::ComputeBallots(_, _) => {
                unreachable!("ComputeBallots is a test-only composition action")
            }
        }
    }

    /// Compute the successor of `state` under `turn` — the one real transition.
    /// Same two-clause guard as the crypto harness: idle cycles are discarded
    /// before durable updates (observation-timing compression), productive
    /// cycles only if they changed nothing (`next == *state`).
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
            }
            Turn::PostBallots => {
                let pk_hash = self.public_key_hash_on(state)?;
                let token = format!("ballots:{pk_hash:?}").into_bytes();
                board.push(ProtocolMessage::<C>::ballots(
                    &self.manager,
                    DATE,
                    self.configuration_hash,
                    pk_hash,
                    self.mixing_trustees.clone(),
                    &token,
                ));
            }
        }

        next.board = board.snapshot().iter().map(|m| m.ser()).collect();
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
            trustees: (0..TRUSTEES)
                .map(|_| TrusteeDurable {
                    committed: Vec::new(),
                    own_posts: Vec::new(),
                })
                .collect(),
            halts: Vec::new(),
        }]
    }

    /// The lookahead: only turns that actually move the system are offered.
    fn actions(&self, state: &Self::State, actions: &mut Vec<Self::Action>) {
        // A halted system takes no further steps.
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
        vec![
            // Safety, on every reachable state.
            Property::<Self>::always("no trustee halts", |_, state| state.halts.is_empty()),
            // Liveness, in its strong form: on EVERY path the protocol
            // completes — each trustee publishes its plaintexts. Sound here,
            // unlike in the crypto harness, because the exploration is
            // exhaustive (deterministic edges + dedup) and acyclic (the board
            // only grows), with no depth cap.
            Property::<Self>::eventually("protocol completes", |_, state| {
                let plaintexts = state
                    .board
                    .iter()
                    .filter(|bytes| {
                        ProtocolMessage::<C>::deser(bytes)
                            .map(|m| m.message_type == MessageType::Plaintexts)
                            .unwrap_or(false)
                    })
                    .count();
                plaintexts == TRUSTEES
            }),
        ]
    }
}

/// Explore ALL interleavings of a two-trustee run over the real datalog with
/// symbolic artifacts. Not `#[ignore]`d: with tokens instead of crypto this is
/// meant to be fast enough for the ordinary test suite — that speed is part of
/// what the test demonstrates.
#[test]
fn model_check_symbolic_two_trustees() {
    let model = SymbolicModel::new();
    let checker = model.checker().threads(1).spawn_bfs().join();
    checker.assert_properties();
    println!("explored {} unique states", checker.unique_state_count());
}
