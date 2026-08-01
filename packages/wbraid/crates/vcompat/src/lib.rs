// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! # vcompat — Verificatum wire-format compatibility
//!
//! An experimental crate supporting the investigation written up in
//! `VERIFICATUM.md`: can Verificatum's `vmnv` be reused, unmodified, as an
//! independent verifier for braid?
//!
//! The finding that motivates this crate is that braid and Verificatum already
//! implement the *same* mix-net — El Gamal, the Terelius–Wikström proof of
//! shuffle, threshold decryption — but disagree on every byte of the
//! **Fiat–Shamir transcript**. Since `vmnv` recomputes each challenge from its
//! own serialization, no post-hoc converter can make braid's existing proofs
//! verify; braid must instead be able to *emit* proofs the way VMN does. This
//! crate is that emitting/parsing layer, built bottom-up and validated against a
//! real VMN-generated corpus at each step.
//!
//! Stage 1 (this code): byte trees and the arithmetic encodings.
//! Stage 2 (next): VMN's hash, PRG, random oracles, and independent generators —
//! the go/no-go gate for the whole approach.
//!
//! ## Module map
//!
//! | Module | Role | Spec |
//! |---|---|---|
//! | [`bytetree`] | the universal serialization format, strict parsing | VMNV §4 |
//! | [`arithm`] | integers, field elements, curve points, product arrays | VMNV §6 |
//! | [`marshal`] | group descriptors, `comment::hex` marshalling, P-256 | VMNV §6.7 |
//!
//! ## Two encoding traps
//!
//! Both were found by hexdumping real VMN output rather than by reading prose,
//! and both are silent if got wrong:
//!
//! 1. **Integers are signed.** A positive value whose top bit would be set gains
//!    a leading `0x00`, so P-256 coordinates and scalars are **33 bytes, not
//!    32** ([`arithm`]).
//! 2. **Product arrays are transposed.** An array of width-`w` elements is
//!    stored as `w` arrays of components, not as a list of tuples
//!    ([`arithm::product_array`]).

pub mod arithm;
pub mod bytetree;
pub mod error;
pub mod marshal;

pub use bytetree::ByteTree;
pub use error::{Error, Result};
