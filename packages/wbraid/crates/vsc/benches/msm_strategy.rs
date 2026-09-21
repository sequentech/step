// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: Apache-2.0

//! Multi-exponentiation strategy micro-benchmark (ristretto255).
//!
//! Decides, from data, how `GroupElement::multi_exp` /
//! `vartime_multi_exp` should be implemented before the shuffle is wired to
//! them. It compares, on identical `(bases, scalars)`:
//!
//! - `naive_par` — the current shuffle pattern: `par_iter().map(exp).reduce(mul)`,
//!   N full exponentiations across the rayon pool. The baseline every MSM must
//!   beat.
//! - `ct_single` / `vt_single` — one dalek `multiscalar_mul` /
//!   `vartime_multiscalar_mul` call (Straus below 190 points, Pippenger above),
//!   **single-threaded**.
//! - `ct_chunk_t` / `vt_chunk_t` — the input split into `num_threads` chunks,
//!   one dalek MSM per chunk on rayon, partials summed.
//! - `ct_chunk_4t` / `vt_chunk_4t` — same, `4·num_threads` chunks (finer
//!   granularity for work-steal balance).
//!
//! The premise under test (PERFORMANCE.md §1): a *single* dalek MSM may lose to the
//! parallel-naive baseline on a many-core machine, while a *chunked* dalek MSM
//! beats both. The winning (strategy, chunk-count) is what the ristretto
//! override should adopt.
//!
//! ```text
//! cargo bench -p vsc --bench msm_strategy
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::missing_docs_in_private_items,
    missing_docs
)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use cryptography::context::Context;
use cryptography::context::RistrettoCtx as RCtx;
use curve25519_dalek::RistrettoPoint;
use curve25519_dalek::Scalar as DalekScalar;
use curve25519_dalek::traits::{Identity, MultiscalarMul, VartimeMultiscalarMul};
use rayon::prelude::*;

const SIZES: &[usize] = &[1_000, 10_000, 100_000];

/// Extract dalek-native inputs once, so the benches time MSM, not conversion.
fn inputs(n: usize) -> (Vec<RistrettoPoint>, Vec<DalekScalar>) {
    let bases = (0..n).map(|_| RCtx::random_element().0).collect();
    let scalars = (0..n).map(|_| RCtx::random_scalar().0).collect();
    (bases, scalars)
}

/// The current shuffle pattern: N full exps across the pool, then a product.
fn naive_par(bases: &[RistrettoPoint], scalars: &[DalekScalar]) -> RistrettoPoint {
    bases
        .par_iter()
        .zip(scalars.par_iter())
        .map(|(b, s)| b * s)
        .reduce(RistrettoPoint::identity, |a, b| a + b)
}

/// `n_chunks` dalek MSMs on the pool, partials summed. `n_chunks == 1` is the
/// single-call strategy.
fn chunked(
    bases: &[RistrettoPoint],
    scalars: &[DalekScalar],
    n_chunks: usize,
    vartime: bool,
) -> RistrettoPoint {
    let cs = bases.len().div_ceil(n_chunks.max(1));
    bases
        .par_chunks(cs)
        .zip(scalars.par_chunks(cs))
        .map(|(bc, sc)| {
            if vartime {
                RistrettoPoint::vartime_multiscalar_mul(sc.iter(), bc.iter())
            } else {
                RistrettoPoint::multiscalar_mul(sc.iter(), bc.iter())
            }
        })
        .reduce(RistrettoPoint::identity, |a, b| a + b)
}

fn bench(c: &mut Criterion) {
    let threads = rayon::current_num_threads();
    let mut g = c.benchmark_group("msm");
    // The big cells are ~100s of ms; keep criterion's run bounded.
    g.sample_size(20).measurement_time(Duration::from_secs(8));

    for &n in SIZES {
        let (bases, scalars) = inputs(n);
        let cases: &[(&str, usize, bool)] = &[
            ("ct_single", 1, false),
            ("ct_chunk_t", threads, false),
            ("ct_chunk_4t", threads * 4, false),
            ("vt_single", 1, true),
            ("vt_chunk_t", threads, true),
            ("vt_chunk_4t", threads * 4, true),
        ];

        g.bench_with_input(BenchmarkId::new("naive_par", n), &n, |b, _| {
            b.iter(|| black_box(naive_par(&bases, &scalars)));
        });
        for &(name, nch, vt) in cases {
            g.bench_with_input(BenchmarkId::new(name, n), &n, |b, _| {
                b.iter(|| black_box(chunked(&bases, &scalars, nch, vt)));
            });
        }
    }
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
