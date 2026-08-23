// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Free & Fair
// See LICENSE.md for details

//! Shuffle scaling benchmark
//!
//! Standalone (non-Bencher) tool for measuring how the Terelius-Wikstrom
//! shuffle proof (see [`cryptography::zkp::shuffle::Shuffler`]) scales with
//! ballot count `N` and ciphertext width `W`, and for benchmarking the two
//! parallel fold strategies behind the crate's `bounded-combine` feature
//! against each other.
//!
//! Unlike `benches/shuffle.rs`, this binary runs one `(count, width)` cell
//! exactly once under direct control (no `test::Bencher` auto-calibration,
//! which makes run time unpredictable for expensive cells), so a driver
//! script or a shell loop can sweep a grid and collect the CSV lines.
//!
//! # Usage
//!
//! ```text
//! cargo run --release --example shuffle_scaling -- <count> <width>
//! ```
//!
//! - `count`: number of ballots (ciphertexts) to shuffle.
//! - `width`: ciphertext width `W`. Must be one of the widths compiled into
//!   [`SUPPORTED_WIDTHS`].
//!
//! To compare the fold strategies, run the same cell with and without the
//! `bounded-combine` feature; the compiled-in strategy is recorded in the
//! CSV line so result files cannot get mixed up:
//!
//! ```text
//! cargo run --release --example shuffle_scaling -- 10000 30
//! cargo run --release --example shuffle_scaling --features bounded-combine -- 10000 30
//! ```
//!
//! On success, prints exactly one CSV line to stdout:
//!
//! ```text
//! count,width,fold,prove_ms,verify_ms,ciphertext_size_of_bytes,ciphertext_serialized_bytes
//! ```
//!
//! `fold` is `reduce` (the default strategy) or `bounded`. Both byte counts
//! are reported because they answer different questions and differ by about
//! 5x for Ristretto: the first is in-memory footprint (what explains
//! resident size), the second is encoded width (what explains storage and
//! transfer). See [`run`].
//!
//! All diagnostics (usage errors, unsupported width, panics) go to stderr,
//! so a driver can rely on stdout containing only the CSV line.

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
use cryptography::cryptosystem::elgamal::Ciphertext;
use cryptography::cryptosystem::elgamal::KeyPair;
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::FSerializable;
use cryptography::zkp::shuffle::Shuffler;
use rayon::prelude::*;

/// Stack size for rayon's pool threads.
///
/// The default fold strategy (rayon's recursive `reduce`) has stack use that
/// grows with `N`, `W` and run-time work stealing: measured on Windows x64,
/// `W = 100` needs 4 MiB at `N = 100`, 8 MiB at `N = 1,000` and 16 MiB at
/// `N = 10,000` -- overflowing default-sized pool threads well inside this
/// tool's parameter range. A fixed, generous reserve keeps that strategy
/// benchmarkable at every cell without per-run tuning; thread stacks are
/// committed page by page as touched, so the unused headroom costs address
/// space, not resident memory. The `bounded-combine` strategy does not need
/// it, and is unaffected by it.
const POOL_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Ciphertext widths `W` that this binary is able to instantiate. `W` is a
/// Rust const generic parameter, so it cannot be threaded through from a
/// runtime `usize` without an explicit dispatch over the supported literal
/// values -- see [`dispatch_width`].
const SUPPORTED_WIDTHS: &[usize] = &[1, 2, 3, 5, 10, 20, 30, 50, 75, 100];

/// Generate `count` random messages of width `W`, encrypt them, shuffle
/// them, and verify the resulting proof.
///
/// Returns `(prove_ms, verify_ms, size_of_bytes, serialized_bytes)`.
///
/// The two byte counts answer different questions and differ by roughly 5x for
/// Ristretto, so both are reported rather than one being picked:
///
/// - `size_of_bytes` is the in-memory footprint of one ciphertext. An element
///   is a `RistrettoPoint`, which dalek stores in extended coordinates -- four
///   field elements of five `u64` limbs, so 160 bytes -- giving `2 * W * 160`.
///   This is the number that explains the process's resident size.
/// - `serialized_bytes` is the encoded width, `Ciphertext::size_bytes()`, which
///   for Ristretto is a 32-byte compressed point per element and so `2 * W *
///   32`. This is the number that explains board storage and transfer cost.
///
/// Taken from the serialization trait rather than computed as `2 * W * 32`, so
/// it stays correct for a backend whose encoded element is not 32 bytes.
///
/// # Panics
///
/// Panics if ciphertext/generator construction, shuffling, or verification
/// fail, or if the produced proof does not verify -- any of these indicate
/// a bug in the library under test, not a usage error.
fn run<C: Context, const W: usize>(count: usize) -> (f64, f64, usize, usize) {
    let keypair: KeyPair<C> = KeyPair::generate();

    let messages: Vec<[C::Element; W]> = (0..count)
        .into_par_iter()
        .map(|_| array::from_fn(|_| C::random_element()))
        .collect();
    let ciphertexts: Vec<Ciphertext<C, W>> =
        messages.par_iter().map(|m| keypair.encrypt(m)).collect();

    let generators = C::G::ind_generators(count, &vec![]).expect("ind_generators failed");
    let shuffler = Shuffler::<C, W>::new(generators, keypair.pkey);

    let prove_start = Instant::now();
    let (permuted_ciphertexts, proof) = shuffler
        .shuffle(&ciphertexts, &vec![])
        .expect("shuffle (prove) failed");
    let prove_ms = prove_start.elapsed().as_secs_f64() * 1000.0;

    let verify_start = Instant::now();
    let ok = shuffler
        .verify(&ciphertexts, &permuted_ciphertexts, &proof, &vec![])
        .expect("verify failed");
    let verify_ms = verify_start.elapsed().as_secs_f64() * 1000.0;

    assert!(ok, "shuffle proof did not verify");

    (
        prove_ms,
        verify_ms,
        std::mem::size_of::<Ciphertext<C, W>>(),
        Ciphertext::<C, W>::size_bytes(),
    )
}

/// Dispatch a runtime `width` value to a monomorphized call of
/// [`run::<RCtx, W>`] for one of a fixed set of literal `W` values.
///
/// Exits the process with a diagnostic on stderr if `width` is not one of
/// the supported literals.
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

/// Parse a required positional CLI argument, exiting with a usage message
/// on failure.
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

/// Print usage information to stderr and exit with status 1.
fn print_usage_and_exit(argv0: &str) -> ! {
    eprintln!("Usage: {argv0} <count> <width>");
    eprintln!("  count: number of ballots to shuffle");
    eprintln!("  width: ciphertext width W; supported: {SUPPORTED_WIDTHS:?}");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let count: usize = parse_arg(&args, 1, "count");
    let width: usize = parse_arg(&args, 2, "width");

    if !SUPPORTED_WIDTHS.contains(&width) {
        eprintln!("error: unsupported width {width}; supported widths are {SUPPORTED_WIDTHS:?}");
        std::process::exit(1);
    }

    // The default fold strategy's deep recursion runs on rayon's pool
    // threads, so the pool is what needs the headroom -- see
    // [`POOL_STACK_BYTES`].
    rayon::ThreadPoolBuilder::new()
        .stack_size(POOL_STACK_BYTES)
        .build_global()
        .expect("failed to configure rayon's global thread pool");

    let fold = if cfg!(feature = "bounded-combine") {
        "bounded"
    } else {
        "reduce"
    };

    eprintln!("running shuffle scaling: count={count} width={width} fold={fold}");

    let (prove_ms, verify_ms, size_of_bytes, serialized_bytes) =
        dispatch_width!(width, count, [1, 2, 3, 5, 10, 20, 30, 50, 75, 100]);

    println!("{count},{width},{fold},{prove_ms},{verify_ms},{size_of_bytes},{serialized_bytes}");
}
