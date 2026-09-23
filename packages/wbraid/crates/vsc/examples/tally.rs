// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: Apache-2.0

//! The global target: critical-path latency of one tally.
//!
//! `examples/targets.rs` times the five stages of a tally in isolation. This
//! tool composes them the way the protocol does — a **replay** of one tally
//! from the posted ballot list to plaintexts, with real data flow between the
//! stages, for a quorum of `Q` trustees acting in turn — and times each stage
//! on the critical path. Each trustee is its own machine in production, so
//! work that parties do concurrently counts once, and work a party does
//! before the next can proceed adds up:
//!
//! ```text
//! T(Q) = 2·Strip + Σ_k (Prove_k + Verify_k) + max_i PartialDecrypt_i + Combine
//! V(Q) =   Strip + Σ_k Verify_k                                     + Combine   (external verifier)
//! ```
//!
//! - **First mix.** Its producer strips the ballot list (Naor-Yung verify and
//!   strip) to obtain `L₀` and proves its shuffle. Its verifier recomputes `L₀`
//!   itself — the mix's input is the ballot list, `L₀` is never posted — then
//!   verifies the proof. Two strips are on the path because the verifier strips
//!   when the mix arrives (braid's `mix_input_ciphertexts`), not eagerly.
//! - **Mixes 2..Q.** Each mixer verifies the mix it received and proves its
//!   own. Non-adjacent verifications happen as mixes are posted, concurrently
//!   with the chain, so only the adjacent one is on the path.
//! - **Decryption.** Every trustee computes its partial decryption
//!   concurrently, so the slowest one gates the next step; whoever combines
//!   verifies all `Q` partials in series (`combine`), as braid's
//!   `ComputePlaintexts` does once the last partial is posted.
//!
//! Each prove and verify derives fresh independent generators, as braid does
//! per mix. Network is out of scope. Serialization — the producer encoding
//! each posted message and its consumer decoding it — is on the path too and
//! is measured behind `--ser`, off by default; the consumer then works from
//! the decoded copies. `V(Q)` never includes it.
//!
//! The replay is a model of braid's *current* schedule over `vsc`'s
//! primitives: if braid moves a check off the path (eager strip, eager
//! partial verification), this composition changes with it.
//!
//! ```text
//! cargo run --release --example tally -- <count> <width> <quorum> [--ser]
//! ```
//!
//! CSV (stdout): `count,width,quorum,ser,strip_prod_ms,strip_ver_ms,prove_ms,verify_ms,partial_ms,combine_ms,ser_ms,t_ms,v_ms`
//! — `prove_ms`/`verify_ms` summed over the `Q` mixes, `partial_ms` the slowest
//! trustee's, `ser` 0/1. The stage breakdown with shares of `T` goes to stderr.
//! The DKG (`Q` of `Q`) and the ballot encryption are untimed setup; the
//! decrypted plaintexts are checked against the encrypted messages.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
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
    AttributedDecryption, PartialDecryption, ParticipantPosition, Recipient, combine,
};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::zkp::shuffle::{ShuffleProof, Shuffler};
use rayon::prelude::*;

const DKG_CTX: &[u8] = b"tally dkg context";
const ENC_CTX: &[u8] = b"tally encryption context";
const SHUFFLE_CTX: &[u8] = b"tally shuffle context";
const DEC_CTX: &[u8] = b"tally decryption context";

/// Ciphertext widths `W` this binary can instantiate (`W` is const generic).
const SUPPORTED_WIDTHS: &[usize] = &[1, 2, 3, 5, 10];
/// Quorum sizes `Q` this binary can instantiate (`Q` is const generic; the
/// committee is `Q` of `Q`).
const SUPPORTED_QUORUMS: &[usize] = &[2, 3, 4, 5, 7];

type Ciphertexts<C, const W: usize> = Vec<elgamal::Ciphertext<C, W>>;

/// Run `f`, returning its result and its wall-clock in milliseconds.
fn timed<R>(f: impl FnOnce() -> R) -> (R, f64) {
    let start = Instant::now();
    let result = f();
    (result, start.elapsed().as_secs_f64() * 1000.0)
}

/// The timed stages of one replay, in milliseconds.
struct Stages {
    strip_prod: f64,
    strip_ver: f64,
    prove: Vec<f64>,
    verify: Vec<f64>,
    partial: Vec<f64>,
    combine: f64,
    ser: f64,
}

impl Stages {
    fn total(&self) -> f64 {
        self.strip_prod
            + self.strip_ver
            + self.prove.iter().sum::<f64>()
            + self.verify.iter().sum::<f64>()
            + self.partial_max()
            + self.combine
            + self.ser
    }

    fn verifier_total(&self) -> f64 {
        self.strip_ver + self.verify.iter().sum::<f64>() + self.combine
    }

    fn partial_max(&self) -> f64 {
        self.partial.iter().copied().fold(0.0, f64::max)
    }
}

/// A posted message on its way to a consumer: with `ser`, encoded and decoded
/// (timed into `ser_ms`) so the consumer works from the decoded copy;
/// otherwise cloned.
fn post<M: Serializable + Deserializable + Clone>(message: &M, ser: bool, ser_ms: &mut f64) -> M {
    if !ser {
        return message.clone();
    }
    let (decoded, elapsed) = timed(|| M::deser(&message.ser()).unwrap());
    *ser_ms += elapsed;
    decoded
}

/// Untimed setup: the committee's keys and the ballot list.
struct Setup<C: Context, const W: usize, const Q: usize> {
    recipients: [Recipient<C, Q, Q>; Q],
    joint_pk: elgamal::PublicKey<C>,
    ny_pk: naoryung::PublicKey<C>,
    messages: Vec<[C::Element; W]>,
    ballots: Vec<naoryung::Ciphertext<C, W>>,
}

fn setup<C: Context, const W: usize, const Q: usize>(count: usize) -> Setup<C, W, Q> {
    // DKG: a committee of Q, threshold Q.
    let dealers: [Dealer<C, Q, Q>; Q] = array::from_fn(|_| Dealer::generate());
    let mut joint_pk: Option<elgamal::PublicKey<C>> = None;
    let recipients: [Recipient<C, Q, Q>; Q] = array::from_fn(|i| {
        let position = ParticipantPosition::from_usize(i + 1);
        let shares: [VerifiableShare<C, Q>; Q] = dealers.clone().map(|d| {
            d.get_verifiable_shares(DKG_CTX)
                .unwrap()
                .for_recipient(&position)
        });
        let (recipient, pk, _vks) = Recipient::from_shares(position, &shares, DKG_CTX).unwrap();
        joint_pk.get_or_insert(pk);
        recipient
    });
    let joint_pk = joint_pk.unwrap();

    // Ballots: Naor-Yung ciphertexts of random messages under the joint key.
    let ny_pk = naoryung::PublicKey::augment(&joint_pk, ENC_CTX).unwrap();
    let (messages, ballots) = (0..count)
        .into_par_iter()
        .map(|_| {
            let message: [C::Element; W] = array::from_fn(|_| C::random_element());
            let ballot = ny_pk.encrypt(&message, ENC_CTX).unwrap();
            (message, ballot)
        })
        .unzip();

    Setup {
        recipients,
        joint_pk,
        ny_pk,
        messages,
        ballots,
    }
}

/// The mixing chain: both strips of the first mix, then `Q` rounds of prove
/// and verify. Returns the final list as its verifier holds it.
fn mix_chain<C: Context, const W: usize, const Q: usize>(
    setup: &Setup<C, W, Q>,
    count: usize,
    ser: bool,
    stages: &mut Stages,
) -> Ciphertexts<C, W> {
    // The ballot list is posted once and read by both parties of the first mix.
    let ballots_prod = post(&setup.ballots, ser, &mut stages.ser);
    let ballots_ver = post(&setup.ballots, ser, &mut stages.ser);

    // First mix, producer: strip to L₀. Its verifier: recompute L₀ itself.
    let (mut input, strip_prod) = timed(|| setup.ny_pk.strip_all(ballots_prod, ENC_CTX).unwrap());
    let (mut input_ver, strip_ver) = timed(|| setup.ny_pk.strip_all(ballots_ver, ENC_CTX).unwrap());
    stages.strip_prod = strip_prod;
    stages.strip_ver = strip_ver;
    assert_eq!(input, input_ver, "the two strips must agree on L₀");

    for k in 1..=Q {
        let gen_seed = format!("tally ind_generators seed, mix {k}");

        let ((output, proof), prove) = timed(|| {
            let gens = C::G::ind_generators(count, gen_seed.as_bytes()).unwrap();
            let shuffler = Shuffler::<C, W>::new(gens, setup.joint_pk.clone());
            shuffler.shuffle(&input, SHUFFLE_CTX).unwrap()
        });
        stages.prove.push(prove);

        let output_ver: Ciphertexts<C, W> = post(&output, ser, &mut stages.ser);
        let proof_ver: ShuffleProof<C, W> = post(&proof, ser, &mut stages.ser);

        let (ok, verify) = timed(|| {
            let gens = C::G::ind_generators(count, gen_seed.as_bytes()).unwrap();
            let shuffler = Shuffler::<C, W>::new(gens, setup.joint_pk.clone());
            shuffler
                .verify(&input_ver, &output_ver, &proof_ver, SHUFFLE_CTX)
                .unwrap()
        });
        stages.verify.push(verify);
        assert!(ok, "mix {k}: shuffle proof did not verify");

        input = output;
        input_ver = output_ver;
    }
    input_ver
}

/// Decryption: `Q` partials computed concurrently (the slowest gates), posted,
/// then verified in series and combined by one party.
fn decrypt<C: Context, const W: usize, const Q: usize>(
    setup: &Setup<C, W, Q>,
    final_list: &[elgamal::Ciphertext<C, W>],
    ser: bool,
    stages: &mut Stages,
) -> Vec<[C::Element; W]> {
    let partials: Vec<PartialDecryption<C, W>> = setup
        .recipients
        .iter()
        .map(|recipient| {
            let (partial, elapsed) =
                timed(|| recipient.partial_decrypt(final_list, DEC_CTX).unwrap());
            stages.partial.push(elapsed);
            partial
        })
        .collect();
    let posted: Vec<PartialDecryption<C, W>> = partials
        .iter()
        .map(|partial| post(partial, ser, &mut stages.ser))
        .collect();
    let contributions: [AttributedDecryption<C, W, Q>; Q] = array::from_fn(|i| {
        AttributedDecryption::new(
            posted[i].clone(),
            setup.recipients[i].get_position().clone(),
            setup.recipients[i].get_verification_key().clone(),
        )
    });

    let (plaintexts, elapsed) =
        timed(|| combine::<C, Q, Q, W>(final_list, &contributions, DEC_CTX).unwrap());
    stages.combine = elapsed;
    plaintexts
}

/// One `(count, width, quorum)` replay.
fn run<C: Context, const W: usize, const Q: usize>(count: usize, ser: bool) -> Stages {
    let setup = setup::<C, W, Q>(count);
    let mut stages = Stages {
        strip_prod: 0.0,
        strip_ver: 0.0,
        prove: Vec::with_capacity(Q),
        verify: Vec::with_capacity(Q),
        partial: Vec::with_capacity(Q),
        combine: 0.0,
        ser: 0.0,
    };

    let final_list = mix_chain(&setup, count, ser, &mut stages);
    let plaintexts = decrypt(&setup, &final_list, ser, &mut stages);

    // Check (untimed): the plaintexts are the encrypted messages, permuted.
    let mut expected: Vec<Vec<u8>> = setup.messages.par_iter().map(Serializable::ser).collect();
    let mut got: Vec<Vec<u8>> = plaintexts.par_iter().map(Serializable::ser).collect();
    expected.par_sort_unstable();
    got.par_sort_unstable();
    assert_eq!(
        got, expected,
        "decrypted plaintexts are not the encrypted messages"
    );

    stages
}

fn unsupported(width: usize, quorum: usize) -> ! {
    eprintln!(
        "error: unsupported width {width} or quorum {quorum}; supported widths {SUPPORTED_WIDTHS:?}, quorums {SUPPORTED_QUORUMS:?}"
    );
    std::process::exit(1);
}

/// Dispatch a runtime `quorum` for a fixed width to a monomorphized [`run`] call.
macro_rules! dispatch_quorum {
    ($w:literal, $quorum:expr, $count:expr, $ser:expr, [$($q:literal),+ $(,)?]) => {
        match $quorum {
            $($q => run::<RCtx, $w, $q>($count, $ser),)+
            q => unsupported($w, q),
        }
    };
}

/// Dispatch runtime `(width, quorum)`: one arm per width, each expanding the
/// quorum list.
macro_rules! dispatch {
    ($width:expr, $quorum:expr, $count:expr, $ser:expr, [$($w:literal),+ $(,)?], $quorums:tt) => {
        match $width {
            $($w => dispatch_quorum!($w, $quorum, $count, $ser, $quorums),)+
            w => unsupported(w, $quorum),
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
    eprintln!("Usage: {argv0} <count> <width> <quorum> [--ser]");
    eprintln!("  count:  number of ballots (ciphertexts)");
    eprintln!("  width:  ciphertext width W; supported: {SUPPORTED_WIDTHS:?}");
    eprintln!("  quorum: trustees mixing and decrypting; supported: {SUPPORTED_QUORUMS:?}");
    eprintln!("  --ser:  also time the encoding/decoding of each posted message");
    std::process::exit(1);
}

fn seconds(ms: f64) -> String {
    format!("{:7.2} s", ms / 1000.0)
}

fn report(count: usize, width: usize, quorum: usize, ser: bool, stages: &Stages) {
    let total = stages.total();
    let prove: f64 = stages.prove.iter().sum();
    let verify: f64 = stages.verify.iter().sum();
    let partial = stages.partial_max();
    let share = |x: f64| format!("{:5.1}%", 100.0 * x / total);
    let list = |v: &[f64]| {
        v.iter()
            .map(|x| format!("{:.2}", x / 1000.0))
            .collect::<Vec<_>>()
            .join(", ")
    };

    eprintln!(
        "tally N={count} W={width} Q={quorum}: T = {}",
        seconds(total).trim()
    );
    eprintln!(
        "  strip (producer)       {}  {}",
        seconds(stages.strip_prod),
        share(stages.strip_prod)
    );
    eprintln!(
        "  strip (verifier)       {}  {}",
        seconds(stages.strip_ver),
        share(stages.strip_ver)
    );
    eprintln!(
        "  prove  x{quorum}              {}  {}   ({})",
        seconds(prove),
        share(prove),
        list(&stages.prove)
    );
    eprintln!(
        "  verify x{quorum}              {}  {}   ({})",
        seconds(verify),
        share(verify),
        list(&stages.verify)
    );
    eprintln!(
        "  partial (slowest of {quorum}) {}  {}   ({})",
        seconds(partial),
        share(partial),
        list(&stages.partial)
    );
    eprintln!(
        "  combine ({quorum} verified)   {}  {}",
        seconds(stages.combine),
        share(stages.combine)
    );
    if ser {
        eprintln!(
            "  ser/deser on the path  {}  {}",
            seconds(stages.ser),
            share(stages.ser)
        );
    }
    eprintln!(
        "  V (external verifier) = {}",
        seconds(stages.verifier_total()).trim()
    );

    println!(
        "{count},{width},{quorum},{},{},{},{prove},{verify},{partial},{},{},{total},{}",
        u8::from(ser),
        stages.strip_prod,
        stages.strip_ver,
        stages.combine,
        stages.ser,
        stages.verifier_total()
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let count: usize = parse_arg(&args, 1, "count");
    let width: usize = parse_arg(&args, 2, "width");
    let quorum: usize = parse_arg(&args, 3, "quorum");
    let ser = match args.get(4).map(String::as_str) {
        None => false,
        Some("--ser") => true,
        Some(other) => {
            eprintln!("error: unexpected argument {other:?}");
            print_usage_and_exit(&args[0]);
        }
    };

    eprintln!("running tally: count={count} width={width} quorum={quorum} ser={ser}");
    let stages = dispatch!(width, quorum, count, ser, [1, 2, 3, 5, 10], [2, 3, 4, 5, 7]);
    report(count, width, quorum, ser, &stages);
}
