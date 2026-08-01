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
//! Everything here is validated against a real VMN-generated corpus, checked in
//! at `testdata/verificatum/`.
//!
//! The crate stops at the wire format and the transcript; it knows nothing about
//! vsc's types. The adapter between the two lives in `braid::vmn`, which keeps
//! the layer whose bytes must match VMN exactly free of the crypto stack.
//!
//! ## Module map
//!
//! | Module | Role | Spec |
//! |---|---|---|
//! | [`bytetree`] | the universal serialization format, strict parsing | VMNV §4 |
//! | [`arithm`] | integers, field elements, curve points, product arrays | VMNV §6 |
//! | [`marshal`] | group descriptors, `comment::hex` marshalling, P-256 | VMNV §6.7 |
//! | [`crypto`] | hash, PRG, random oracles, the global prefix ρ | VMNV §5, §9.3 |
//! | [`generators`] | independent generators via the quadratic-residue walk | VMNV §6.8, §8.2 |
//! | [`lagrange`] | modified Lagrange coefficients and the alpha factor | VMNV §2.2 |
//! | [`error`] | decoding errors | — |
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
pub mod crypto;
pub mod error;
pub mod generators;
pub mod lagrange;
pub mod marshal;

pub use bytetree::ByteTree;
pub use error::{Error, Result};
