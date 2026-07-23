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
//! ## Brain and body (§1)
//!
//! - **Brain** — [`datalog`]: a faithful port of the vs_lift `ascent` rules. Given
//!   the set of verified predicates it has observed, it decides what is proven and
//!   which [`Action`](datalog::Action)s the trustee may take — or raises an error,
//!   which halts the trustee.
//! - **Body** — everything else: message verification and the store
//!   ([`messages`]), the board client and its b4 / persistence seams ([`board`]),
//!   and the action layer that performs the actual cryptography ([`runtime`]).
//! - **Dependencies:** the `cryptography` crate (`vsc`) supplies the cryptographic
//!   primitives the action layer calls (groups, ElGamal, shuffle proofs, DKG,
//!   signatures, serialization); the `b4` crate supplies the message/artifact
//!   types and the wire format ([`b4::messages`]).
//!
//! ## The update-first cycle (§6)
//!
//! A trustee advances by repeating one **update → step → post** cycle. A produced
//! message takes *no* local effect until it loops back from b4 on a later update,
//! so a trustee never advances on its own unconfirmed output:
//!
//! ```text
//! b4 --messages--> BoardClient --> SessionTrustee --> datalog --> actions --> BoardClient --messages--> b4
//!                  verify ->        pure step,         brain:      run crypto,   build + sign
//!                  predicate,       holds keys         predicates  width W       WireMessages
//!                  anti-rewrite,                       -> actions
//!                  persist, store
//! ```
//!
//! - [`BoardClient`](board::BoardClient) is the only slot / `collides()`-aware
//!   layer: it verifies each [`WireMessage`](b4::messages::wire::WireMessage) into
//!   a [`Predicate`](messages::predicate::Predicate) (+ body), runs the anti-rewrite
//!   check against its persisted commitments (§6.2–§6.3), and owns the
//!   [`MessageStore`](messages::store::MessageStore) (§5, §8).
//! - [`SessionTrustee`](runtime::SessionTrustee) is constructed with its secret
//!   keys (§9) and runs a **pure** `step` over the store — no I/O, no state. It is
//!   oblivious to whether the store is one board or a client-composed union of
//!   boards (§8.2).
//! - [`Session`](session::Session) binds one trustee to one board client and drives
//!   the cycle; a multi-trustee harness can run the CPU-bound `step`s in parallel
//!   while keeping the async transport phases sequential.
//!
//! ## Module map
//!
//! | Module | Role | Spec |
//! |---|---|---|
//! | [`messages`] | verify WireMessages -> predicates; the store (datalog EDB) | §3–§5 |
//! | [`datalog`] | the brain: predicates -> actions, or HALT | §7 |
//! | [`runtime`] | [`SessionTrustee`](runtime::SessionTrustee): the pure step + the action/crypto layer | §7.5, §9 |
//! | [`board`] | board client + [`Transport`](board::transport::Transport) / [`Persistence`](board::persistence::Persistence) seams; the b4 board union | §6, §8 |
//! | [`session`] | one trustee bound to one board client; the update-first driver | §6 |
//! | [`protocol`] | const-generic dispatch macros for the ciphertext width `W` | §10.3 |
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
pub mod protocol;
pub mod runtime;
pub mod session;

// Platform-specific modules
#[cfg(feature = "native")]
pub mod native;
#[cfg(feature = "wasm-core")]
pub mod wasm;
