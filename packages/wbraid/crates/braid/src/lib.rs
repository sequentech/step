// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! # braid — a verifiable mixnet trustee
//!
//! braid is the trustee node of a **mixnet**: a committee runs a distributed key
//! generation (DKG), then shuffles (mixes) and threshold-decrypts batches of
//! ciphertexts, publishing every step to an **untrusted** bulletin board (`b4`) so
//! the outcome is independently verifiable.
//!
//! This page is the map from the design documents to the code. The authoritative
//! "what and why" lives in `crates/braid/v0.6_spec.md` (the spec) and
//! `crates/braid/v0.6_decisions.md` (the decision log); testing is described in
//! the repo-root `TESTING.md`. Section references below (§N) point into the spec.
//!
//! ## Within braid: functional core / imperative shell (§1.2)
//!
//! - **The functional core** is [`Trustee::step`](trustee::Trustee::step). It is a
//!   pure function of the board's read view: it performs no I/O and holds no
//!   mutable protocol state between calls — only the fixed secret keys it was
//!   constructed with (§9) — so called twice with the same view it returns the
//!   same result. Internally it reads the board-sourced predicates, runs the
//!   [`datalog`] engine (a faithful port of the vs_lift `ascent` rules) to derive
//!   the enabled [`Action`](datalog::Action)s, and executes each one to produce
//!   signed [`ProtocolMessage`](messages::wire::ProtocolMessage)s — or the
//!   datalog raises an error, halting the trustee.
//! - **The imperative shell** is [`Session`](session::Session) +
//!   [`BoardClient`](board::BoardClient): `Session::advance()` drives `update()`
//!   (fetch from b4) → `step()` (call the pure core) → `post()` (write back to
//!   b4); `BoardClient` owns the b4 transport, the persisted anti-rewrite
//!   commitments, and the in-memory store the core reads.
//! - Consequence: the core carries no memory of its own unconfirmed output
//!   between calls — it only ever sees what the shell hands it as `view`. That is
//!   exactly what makes the update-first / loop-back rule below safe to enforce
//!   entirely in the shell, with no core-side state to advance prematurely.
//!
//! **Dependencies:** the `cryptography` crate (`vsc`) supplies the cryptographic
//! primitives the action layer calls (groups, ElGamal, shuffle proofs, DKG,
//! signatures, serialization); the `b4` crate is the bulletin board braid posts
//! to and fetches from, over the HTTP boundary its `api_types` define
//! ([`board::transport`]).
//!
//! ## The update-first cycle (§6)
//!
//! A trustee advances by repeating one **update → step → post** cycle. A produced
//! message takes *no* local effect until it loops back from b4 on a later update,
//! so a trustee never advances on its own unconfirmed output:
//!
//! ```text
//! b4 --messages--> BoardClient --> Trustee --> datalog --> actions --> BoardClient --messages--> b4
//!                  verify ->        pure step,   brain:      run crypto,   build + sign
//!                  predicate,       holds keys    predicates  width W       ProtocolMessages
//!                  anti-rewrite,                  -> actions
//!                  persist, store
//! ```
//!
//! - [`BoardClient`](board::BoardClient) is the only persistence/completeness-aware
//!   layer: it verifies each [`ProtocolMessage`](messages::wire::ProtocolMessage) into
//!   a [`Predicate`](messages::predicate::Predicate) (+ body), runs the anti-rewrite
//!   completeness gate against its persisted commitments (§6.2–§6.3), and owns the
//!   [`MessageStore`](board::store::MessageStore) (§5, §8). `collides()` itself
//!   stays the datalog's job alone (§5.3).
//! - [`Trustee`](trustee::Trustee) is constructed with its secret keys (§9) and
//!   runs a **pure** `step` over the store — no I/O, no state. It is oblivious to
//!   whether the store is one board or a client-composed union of boards (§8.2).
//! - [`Session`](session::Session) binds one trustee to one board client and drives
//!   the cycle; a multi-trustee harness can run the CPU-bound `step`s in parallel
//!   while keeping the async transport phases sequential.
//!
//! ## Module map
//!
//! | Module | Role | Spec |
//! |---|---|---|
//! | [`messages`] | message/artifact vocabulary: wire format, predicates, signing | §3–§5 |
//! | [`datalog`] | the brain: predicates -> actions, or HALT | §7 |
//! | [`trustee`] | [`Trustee`](trustee::Trustee): the pure step + the action/crypto layer, split by protocol phase | §7.5, §9 |
//! | [`board`] | board client: [`store`](board::store) (EDB), [`verify`](board::verify) (ProtocolMessage -> Predicate), [`Transport`](board::transport::Transport) / [`Persistence`](board::persistence::Persistence) seams; the b4 board union | §6, §8 |
//! | [`session`] | one trustee bound to one board client; the update-first driver | §6 |
//! | [`protocol_manager`] | [`ProtocolManager`](protocol_manager::ProtocolManager): the other protocol participant, authors `Configuration`/`Ballots` | §4.3, §9.6 |
//! | [`dispatch`] | const-generic dispatch macros for the ciphertext width `W` | §10.3 |
//! | [`native`] | native-only: logging + the test harnesses | §2 |
//! | `wasm` (feature `wasm-core`) | wasm bindings + the interactive emulator | §2 (M3) |
//!
//! ## Feature flags
//!
//! - `native` (default) — the native build: SQLite persistence, HTTP transport,
//!   and the native test harnesses.
//! - `wasm-core` — the base wasm build: browser-`fetch` transport, IndexedDB
//!   persistence, and the emulator; no thread pool (so it needs no atomics).
//! - `wasm` — `wasm-core` plus `wasm-bindgen-rayon` for parallel crypto in the
//!   browser (the production wasm build).

extern crate cfg_if;

pub mod board;
pub mod datalog;
pub mod messages;
pub mod dispatch;
pub mod protocol_manager;
pub mod session;
pub mod trustee;

// Platform-specific modules
#[cfg(feature = "native")]
pub mod native;
/// Verificatum interoperability (experimental; see the repo-root `VERIFICATUM.md`).
#[cfg(feature = "native")]
pub mod vmn;
#[cfg(feature = "wasm-core")]
pub mod wasm;
