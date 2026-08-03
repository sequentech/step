// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! # vsvmn — vs <-> vmn interoperability
//!
//! Everything about talking to [Verificatum](https://www.verificatum.org) lives
//! here, so that neither vsc nor braid carries any knowledge of it. The
//! investigation this grew out of is written up in the repo-root
//! `VERIFICATUM.md`.
//!
//! Two directions, both exercised by the tests:
//!
//! - **emit** proofs that Verificatum's `vmnv` — an independently written
//!   verifier, in a different language, sharing no code with us — accepts;
//! - **ingest** proofs Verificatum produced, and verify them here.
//!
//! The two systems already implement the same mix-net and the same
//! Terelius–Wikström shuffle proof; what differs is the Fiat–Shamir transcript.
//! Since each verifier recomputes every challenge from its own serialization, a
//! post-hoc converter cannot work — a proof has to be *produced* the other
//! side's way from the start. That is what this crate is for.
//!
//! ## Layering, which is load-bearing
//!
//! - [`wire`] owns Verificatum's byte format and random oracles and **must not
//!   depend on vsc**. It is the layer whose bytes have to match exactly, and
//!   keeping the crypto stack out of it is what lets it be validated directly
//!   against VMN's own golden vectors — so a mismatch is immediately either a
//!   serialization bug or a cryptography bug, never an unresolved mixture.
//! - vsc owns the cryptography and knows nothing about Verificatum.
//! - **The rest of this crate is the only adapter between them.**
//!
//! Cargo no longer enforces that boundary, now that [`wire`] is a module rather
//! than its own crate. Nothing in it may `use cryptography::…`; if that ever
//! needs to change, the right move is to extract it back out rather than to
//! relax it.
//!
//! ## Scope
//!
//! P-256 only. Verificatum's `ECqPGroup` supports a fixed list of standard
//! curves, and Ristretto255 — braid's default context — is not among them, so
//! only P-256 executions can be checked this way.

pub mod wire;

pub mod challenges;
pub mod decrypt;
pub mod encode;
pub mod generators;
pub mod proof_dir;
