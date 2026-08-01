// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Verificatum interoperability (experimental).
//!
//! Supports the investigation written up in the repo-root `VERIFICATUM.md`:
//! emitting proofs that Verificatum's `vmnv` — an independently written verifier
//! in a different language, sharing no code with braid — will accept.
//!
//! braid and Verificatum already implement the same mix-net and the same
//! Terelius–Wikström shuffle proof; what differs is the Fiat–Shamir transcript.
//! Since `vmnv` recomputes every challenge from its own serialization, a
//! post-hoc converter cannot work — braid has to *produce* the transcript VMN's
//! way. That is what this module is for.
//!
//! ## Layering
//!
//! - `vcompat` owns Verificatum's wire format and random oracles, and depends on
//!   nothing from vsc. It is the layer whose bytes must match exactly, so it is
//!   kept small and free of the crypto stack.
//! - vsc owns the cryptography and knows nothing about Verificatum.
//! - **This module is the only adapter between them.**
//!
//! ## Scope
//!
//! P-256 only. Verificatum's `ECqPGroup` supports a fixed list of standard
//! curves and Ristretto255 — braid's default context — is not among them, so
//! only P-256 executions can be checked this way.

pub mod challenges;
pub mod encode;
pub mod generators;
