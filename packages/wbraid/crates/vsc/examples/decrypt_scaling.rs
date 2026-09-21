// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: Apache-2.0

//! Tally per-ballot crypto scaling benchmark
//!
//! Standalone companion to `shuffle_scaling`, covering the two per-`N` tally
//! costs the shuffle benchmark does not — one from the mixing phase, two from
//! decryption:
//!
//! - **Naor-Yung verify-and-strip** — a *first-mix* cost, not a decryption
//!   one: every quorum trustee runs `NYVerify` and strips the ballots to
//!   ElGamal to form `L_0` before mixing (braid's `mix_input_ciphertexts`).
//!   It appears here because it needs Naor-Yung ballots and this tally-crypto
//!   benchmark is a convenient home for its serial-vs-parallel comparison; the
//!   decryption measurements below do not depend on it.
//! - **`Recipient::partial_decrypt`** — one trustee's factors (`N·W`
//!   exponentiations) plus its single batched proof. **Decryption**.
//! - **`combine`** — verifying `T` contributions' batched proofs and
//!   interpolating the plaintexts. **Decryption**.
//!
//! Decryption operates only on ElGamal ciphertexts (never Naor-Yung), and its
//! cost is the same for `N` ciphertexts under the joint key regardless of
//! whether they came from stripping ballots (`L_0`) or a completed mix
//! (`L_t`) — so the tool encrypts fresh ElGamal ciphertexts under the joint
//! key and decrypts those directly, independent of the strip block above,
//! rather than stripping ballots or running a shuffle. The DKG (fixed at
//! `T = 3, P = 5`) is setup, not measurement: its cost does not depend on `N`.
//!
//! # Usage
//!
//! ```text
//! cargo run --release --example decrypt_scaling -- <count> <width>
//! ```
//!
//! On success, prints exactly one CSV line to stdout:
//!
//! ```text
//! count,width,strip_serial_ms,strip_par_ms,partial_decrypt_ms,combine_ms
//! ```
//!
//! All diagnostics go to stderr, so a driver can rely on stdout containing
//! only the CSV line.

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
use rayon::prelude::*;

/// Decryption threshold for the fixed DKG shape.
const T: usize = 3;
/// Committee size for the fixed DKG shape.
const P: usize = 5;

/// Domain contexts. Arbitrary but fixed: this tool measures cost, not
/// interop, and every run is self-contained (encrypt and decrypt under the
/// same contexts).
const DKG_CTX: &[u8] = b"decrypt_scaling dkg context";
const ENC_CTX: &[u8] = b"decrypt_scaling encryption context";
const PROOF_CTX: &[u8] = b"decrypt_scaling proof context";

/// Ciphertext widths `W` this binary can instantiate (`W` is const generic,
/// so runtime dispatch is over a fixed literal set — see [`dispatch_width`]).
const SUPPORTED_WIDTHS: &[usize] = &[1, 2, 3, 5, 10];

/// Run one `(count, width)` cell.
///
/// Returns `(strip_serial_ms, strip_par_ms, partial_decrypt_ms, combine_ms)`.
fn run<C: Context, const W: usize>(count: usize) -> (f64, f64, f64, f64) {
    // DKG (setup, untimed): P dealers, P recipients, threshold T.
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
    let joint_pk = &recipients[0].1;

    // === First-mix Naor-Yung verify-and-strip (a MIXING-input cost) ===
    // Not decryption: this is the shape of braid's `mix_input_ciphertexts`,
    // which forms L_0 before the first mix. It is the tool's only Naor-Yung
    // use, measured here because it needs Naor-Yung ballots and this is a
    // convenient site for the serial-vs-parallel strip comparison.
    let ny_pk = naoryung::PublicKey::augment(joint_pk, ENC_CTX).unwrap();
    let ballots: Vec<naoryung::Ciphertext<C, W>> = (0..count)
        .into_par_iter()
        .map(|_| {
            let message: [C::Element; W] = array::from_fn(|_| C::random_element());
            ny_pk.encrypt(&message, ENC_CTX).unwrap()
        })
        .collect();

    let ballots_serial = ballots.clone();
    let start = Instant::now();
    let stripped_serial: Vec<elgamal::Ciphertext<C, W>> = ballots_serial
        .into_iter()
        .map(|c| ny_pk.strip(c, ENC_CTX).unwrap())
        .collect();
    let strip_serial_ms = start.elapsed().as_secs_f64() * 1000.0;

    let start = Instant::now();
    let stripped_par: Vec<elgamal::Ciphertext<C, W>> = ballots
        .into_par_iter()
        .map(|c| ny_pk.strip(c, ENC_CTX).unwrap())
        .collect();
    let strip_par_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert!(
        stripped_serial == stripped_par,
        "serial and parallel strip disagree"
    );

    // === Threshold decryption (ElGamal only) ===
    // Encrypt directly under the joint ElGamal key -- no Naor-Yung, no strip.
    // Decryption cost is identical for N ElGamal ciphertexts whether they came
    // from stripping ballots (L_0) or a completed mix (L_t), so the tool skips
    // the mix and decrypts fresh encryptions.
    let ciphertexts: Vec<elgamal::Ciphertext<C, W>> = (0..count)
        .into_par_iter()
        .map(|_| {
            let message: [C::Element; W] = array::from_fn(|_| C::random_element());
            joint_pk.encrypt(&message)
        })
        .collect();

    // Partial decryption: T contributions; the first one timed.
    let mut partial_decrypt_ms = 0.0;
    let contributions: [AttributedDecryption<C, W, P>; T] = array::from_fn(|i| {
        let start = Instant::now();
        let partial = recipients[i].0.partial_decrypt(&ciphertexts, PROOF_CTX).unwrap();
        if i == 0 {
            partial_decrypt_ms = start.elapsed().as_secs_f64() * 1000.0;
        }
        AttributedDecryption::new(
            partial,
            recipients[i].0.get_position().clone(),
            recipients[i].0.get_verification_key().clone(),
        )
    });

    // Combine: verify the T batched proofs and interpolate the plaintexts.
    let start = Instant::now();
    let plaintexts = combine::<C, T, P, W>(&ciphertexts, &contributions, PROOF_CTX).unwrap();
    let combine_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(plaintexts.len(), count, "combine returned a short list");

    (strip_serial_ms, strip_par_ms, partial_decrypt_ms, combine_ms)
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

/// Parse a required positional CLI argument, exiting with a usage message on
/// failure.
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
    eprintln!("  count: number of ballots to strip and decrypt");
    eprintln!("  width: ciphertext width W; supported: {SUPPORTED_WIDTHS:?}");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let count: usize = parse_arg(&args, 1, "count");
    let width: usize = parse_arg(&args, 2, "width");

    eprintln!("running decrypt scaling: count={count} width={width} T={T} P={P}");

    let (strip_serial_ms, strip_par_ms, partial_decrypt_ms, combine_ms) =
        dispatch_width!(width, count, [1, 2, 3, 5, 10]);

    println!(
        "{count},{width},{strip_serial_ms},{strip_par_ms},{partial_decrypt_ms},{combine_ms}"
    );
}
