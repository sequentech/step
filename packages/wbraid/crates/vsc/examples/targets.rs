// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: Apache-2.0

//! Top-level target snapshot benchmark.
//!
//! A black-box measure of the five operator-observable, N-scaling crypto
//! operations of a tally, in production (parallel) form — one CSV line per
//! `(N, W)` cell. Distinct from the optimization-guidance benches
//! (`benches/msm_strategy.rs`, `benches/parallel_tradeoff.rs`), which time
//! sub-primitives to steer implementation; this tool answers "where is
//! top-level performance now?" as a readable snapshot.
//!
//! The five targets:
//!
//! 1. **shuffle prove** — `Shuffler::shuffle` (re-encrypt + permute + the
//!    Terelius-Wikstrom proof), *including* the `ind_generators` derivation,
//!    because braid derives fresh generators per `compute_mix`.
//! 2. **shuffle verify** — `Shuffler::verify`, likewise including a fresh
//!    `ind_generators` (braid derives them per `sign_mix`).
//! 3. **partial decryption** — one trustee's `Recipient::partial_decrypt`
//!    (`N·W` factors plus its single batched proof).
//! 4. **decryption / combine** — `combine`, which verifies all `T` trustees'
//!    batched proofs and Lagrange-interpolates the plaintexts.
//! 5. **Naor-Yung verify-and-strip** — the first-mix ballot ingestion
//!    (`NYVerify` + strip to ElGamal), a mixing-input cost.
//!
//! Scope (Caveat): these time the `vsc` crypto, not the braid trustee actions
//! that wrap them (input deserialization, output-message serialization,
//! Ed25519 signing, board I/O). The Fiat-Shamir transcript serialization — the
//! dominant residual — is included, since it lives inside these calls.
//!
//! The tool uses only public `vsc` entry points present since the fork point,
//! so the same source compiles on the pre-optimization baseline: copy it into
//! a baseline checkout and run `bench` on both for a campaign-wide before/after.
//!
//! ```text
//! cargo run --release --example targets -- <count> <width>
//! ```
//!
//! CSV: `count,width,prove_ms,verify_ms,partial_decrypt_ms,combine_ms,ny_strip_ms,sizeof_bytes,ser_bytes`.
//! `sizeof_bytes` is one ciphertext's in-memory footprint, `ser_bytes` its
//! encoded width. The DKG (fixed `T = 3, P = 5`) is untimed setup.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    missing_docs,
    clippy::missing_docs_in_private_items
)]

use std::array;
use std::time::Instant;

use cryptography::context::Context;
use cryptography::context::RistrettoCtx as RCtx;
use cryptography::cryptosystem::elgamal;
use cryptography::cryptosystem::naoryung;
use cryptography::dkgd::dealer::{Dealer, VerifiableShare};
use cryptography::dkgd::recipient::{
    AttributedDecryption, ParticipantPosition, Recipient, combine,
};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::Serializable;
use cryptography::zkp::shuffle::Shuffler;
use rayon::prelude::*;

/// Decryption threshold and committee size (untimed setup; N-independent).
const T: usize = 3;
const P: usize = 5;

const DKG_CTX: &[u8] = b"targets dkg context";
const ENC_CTX: &[u8] = b"targets encryption context";
const SHUFFLE_CTX: &[u8] = b"targets shuffle context";
const DEC_CTX: &[u8] = b"targets decryption context";
const GEN_SEED: &[u8] = b"targets ind_generators seed";

/// Ciphertext widths `W` this binary can instantiate (`W` is const generic).
const SUPPORTED_WIDTHS: &[usize] = &[1, 2, 3, 5, 10];

/// One `(count, width)` cell. Returns the five target timings (ms) plus the two
/// ciphertext byte counts.
fn run<C: Context, const W: usize>(count: usize) -> (f64, f64, f64, f64, f64, usize, usize) {
    // --- DKG (untimed setup): P dealers, P recipients, threshold T. ---
    let dealers: [Dealer<C, T, P>; P] = array::from_fn(|_| Dealer::generate());
    let recipients: [(Recipient<C, T, P>, elgamal::PublicKey<C>); P] = array::from_fn(|i| {
        let position = ParticipantPosition::from_usize(i + 1);
        let shares: [VerifiableShare<C, T>; P] = dealers.clone().map(|d| {
            d.get_verifiable_shares(DKG_CTX)
                .unwrap()
                .for_recipient(&position)
        });
        let (recipient, joint_pk, _vks) =
            Recipient::from_shares(position, &shares, DKG_CTX).unwrap();
        (recipient, joint_pk)
    });
    let joint_pk = recipients[0].1.clone();

    // ElGamal ciphertexts under the election key, the shuffle/decrypt input
    // (untimed setup).
    let ciphertexts: Vec<elgamal::Ciphertext<C, W>> = (0..count)
        .into_par_iter()
        .map(|_| {
            let message: [C::Element; W] = array::from_fn(|_| C::random_element());
            joint_pk.encrypt(&message)
        })
        .collect();

    // --- (1) shuffle prove (incl. fresh ind_generators, as braid does). ---
    let start = Instant::now();
    let gens = C::G::ind_generators(count, GEN_SEED).unwrap();
    let shuffler = Shuffler::<C, W>::new(gens, joint_pk.clone());
    let (permuted, proof) = shuffler.shuffle(&ciphertexts, SHUFFLE_CTX).unwrap();
    let prove_ms = start.elapsed().as_secs_f64() * 1000.0;

    // --- (2) shuffle verify (fresh ind_generators, as braid does). ---
    let start = Instant::now();
    let gens_v = C::G::ind_generators(count, GEN_SEED).unwrap();
    let shuffler_v = Shuffler::<C, W>::new(gens_v, joint_pk.clone());
    let ok = shuffler_v
        .verify(&ciphertexts, &permuted, &proof, SHUFFLE_CTX)
        .unwrap();
    let verify_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert!(ok, "shuffle proof did not verify");

    // Decryption runs on the mixed output (its cost is identical to L_0).
    // --- (3) partial decryption: T contributions, the first one timed. ---
    let mut partial_decrypt_ms = 0.0;
    let contributions: [AttributedDecryption<C, W, P>; T] = array::from_fn(|i| {
        let start = Instant::now();
        let partial = recipients[i].0.partial_decrypt(&permuted, DEC_CTX).unwrap();
        if i == 0 {
            partial_decrypt_ms = start.elapsed().as_secs_f64() * 1000.0;
        }
        AttributedDecryption::new(
            partial,
            recipients[i].0.get_position().clone(),
            recipients[i].0.get_verification_key().clone(),
        )
    });

    // --- (4) combine: verify the T proofs and interpolate the plaintexts. ---
    let start = Instant::now();
    let plaintexts = combine::<C, T, P, W>(&permuted, &contributions, DEC_CTX).unwrap();
    let combine_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(plaintexts.len(), count, "combine returned a short list");

    // --- (5) first-mix Naor-Yung verify-and-strip (mixing-input cost). ---
    let ny_pk = naoryung::PublicKey::augment(&joint_pk, ENC_CTX).unwrap();
    let ballots: Vec<naoryung::Ciphertext<C, W>> = (0..count)
        .into_par_iter()
        .map(|_| {
            let message: [C::Element; W] = array::from_fn(|_| C::random_element());
            ny_pk.encrypt(&message, ENC_CTX).unwrap()
        })
        .collect();
    let start = Instant::now();
    let stripped: Vec<elgamal::Ciphertext<C, W>> = ballots
        .into_par_iter()
        .map(|c| ny_pk.strip(c, ENC_CTX).unwrap())
        .collect();
    let ny_strip_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(stripped.len(), count, "strip returned a short list");

    let sizeof_bytes = std::mem::size_of::<elgamal::Ciphertext<C, W>>();
    let ser_bytes = elgamal::Ciphertext::<C, W>::new(
        array::from_fn(|_| C::generator()),
        array::from_fn(|_| C::generator()),
    )
    .ser()
    .len();

    (
        prove_ms,
        verify_ms,
        partial_decrypt_ms,
        combine_ms,
        ny_strip_ms,
        sizeof_bytes,
        ser_bytes,
    )
}

/// Dispatch a runtime `width` to a monomorphized [`run::<RCtx, W>`] call.
macro_rules! dispatch_width {
    ($width:expr, $count:expr, [$($w:literal),+ $(,)?]) => {
        match $width {
            $($w => run::<RCtx, $w>($count),)+
            other => {
                eprintln!(
                    "error: unsupported width {other}; supported widths are {SUPPORTED_WIDTHS:?}"
                );
                std::process::exit(1);
            }
        }
    };
}

fn parse_arg<T: std::str::FromStr>(args: &[String], index: usize, name: &str) -> T {
    let Some(raw) = args.get(index) else {
        eprintln!("error: missing required argument <{name}>");
        print_usage_and_exit(&args[0]);
    };
    raw.parse().unwrap_or_else(|_| {
        eprintln!("error: could not parse <{name}> from {raw:?}");
        print_usage_and_exit(&args[0]);
    })
}

fn print_usage_and_exit(argv0: &str) -> ! {
    eprintln!("Usage: {argv0} <count> <width>");
    eprintln!("  count: number of ballots (ciphertexts)");
    eprintln!("  width: ciphertext width W; supported: {SUPPORTED_WIDTHS:?}");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let count: usize = parse_arg(&args, 1, "count");
    let width: usize = parse_arg(&args, 2, "width");

    eprintln!("running targets: count={count} width={width} T={T} P={P}");

    let (prove, verify, partial, combine_ms, ny_strip, sizeof, ser) =
        dispatch_width!(width, count, [1, 2, 3, 5, 10]);

    println!("{count},{width},{prove},{verify},{partial},{combine_ms},{ny_strip},{sizeof},{ser}");
}
