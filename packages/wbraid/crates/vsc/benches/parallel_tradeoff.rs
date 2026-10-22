// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: Apache-2.0

//! Serial-vs-parallel micro-benchmark for the shuffle/dkgd per-element loops.
//!
//! Rayon is not free: each `par_iter` pays split and work-steal overhead and
//! commits worker-thread stack. For cheap per-element work (scalar field ops
//! are nanoseconds) that overhead can dominate, and the parallelism then costs
//! clarity for no speed. This bench measures each *operation shape* used by
//! `zkp::shuffle` and `dkgd::recipient`, serial vs parallel, so the choice at
//! each site is made from data rather than reflex.
//!
//! Criterion (not hand-timed single shots) because the cheap shapes are close
//! calls, exactly where a naive timer gives the wrong verdict.
//!
//! ```text
//! cargo bench -p vsc --bench parallel_tradeoff
//! ```
//!
//! Reading it: within each `group/size` pair, compare `serial` vs `parallel`.
//! Parallel wins clearly on point exponentiation and hashing; the question the
//! bench exists to answer is the cheap shapes (scalar arithmetic, point
//! products), where it often does not.

#![allow(
    clippy::unwrap_used,
    clippy::missing_docs_in_private_items,
    missing_docs
)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use cryptography::context::Context;
use cryptography::context::RistrettoCtx as RCtx;
use cryptography::traits::groups::{CryptographicGroup, GroupElement, GroupScalar};
use rayon::prelude::*;

/// Ballot counts spanning the realistic range (small tally to large election).
const SIZES: &[usize] = &[1_000, 10_000, 100_000];

type S = <RCtx as Context>::Scalar;
type E = <RCtx as Context>::Element;
type G = <RCtx as Context>::G;

fn scalars(n: usize) -> Vec<S> {
    (0..n).map(|_| RCtx::random_scalar()).collect()
}
fn elements(n: usize) -> Vec<E> {
    (0..n).map(|_| RCtx::random_element()).collect()
}

/// scalar RNG loop — `b_n`, `beta_n`/`epsilon_n`.
fn bench_scalar_rng(c: &mut Criterion) {
    let mut g = c.benchmark_group("scalar_rng");
    for &n in SIZES {
        g.bench_with_input(BenchmarkId::new("serial", n), &n, |b, &n| {
            b.iter(|| {
                let v: Vec<S> = (0..n).map(|_| RCtx::random_scalar()).collect();
                black_box(v)
            });
        });
        g.bench_with_input(BenchmarkId::new("parallel", n), &n, |b, &n| {
            b.iter(|| {
                let v: Vec<S> = (0..n)
                    .into_par_iter()
                    .map(|_| RCtx::random_scalar())
                    .collect();
                black_box(v)
            });
        });
    }
    g.finish();
}

/// scalar mul+add loop — `k_b_n`, `k_e_n` (`k = v*x + y`).
fn bench_scalar_mul_add(c: &mut Criterion) {
    let v = RCtx::random_scalar();
    let mut g = c.benchmark_group("scalar_mul_add");
    for &n in SIZES {
        let (xs, ys) = (scalars(n), scalars(n));
        g.bench_with_input(BenchmarkId::new("serial", n), &n, |b, _| {
            b.iter(|| {
                let out: Vec<S> = xs
                    .iter()
                    .zip(ys.iter())
                    .map(|(x, y)| v.mul(x).add(y))
                    .collect();
                black_box(out)
            });
        });
        g.bench_with_input(BenchmarkId::new("parallel", n), &n, |b, _| {
            b.iter(|| {
                let out: Vec<S> = xs
                    .par_iter()
                    .zip(ys.par_iter())
                    .map(|(x, y)| v.mul(x).add(y))
                    .collect();
                black_box(out)
            });
        });
    }
    g.finish();
}

/// scalar inner-product reduce — `a = Σ r_i·e'_i`.
fn bench_scalar_inner_product(c: &mut Criterion) {
    let mut g = c.benchmark_group("scalar_inner_product");
    for &n in SIZES {
        let (xs, ys) = (scalars(n), scalars(n));
        g.bench_with_input(BenchmarkId::new("serial", n), &n, |b, _| {
            b.iter(|| {
                let acc = xs
                    .iter()
                    .zip(ys.iter())
                    .map(|(x, y)| x.mul(y))
                    .fold(S::zero(), |a, n| a.add(&n));
                black_box(acc)
            });
        });
        g.bench_with_input(BenchmarkId::new("parallel", n), &n, |b, _| {
            b.iter(|| {
                let acc = xs
                    .par_iter()
                    .zip(ys.par_iter())
                    .map(|(x, y)| x.mul(y))
                    .reduce(S::zero, |a, n| a.add(&n));
                black_box(acc)
            });
        });
    }
    g.finish();
}

/// scalar product reduce — `e_n_fold = Π e_i`.
fn bench_scalar_product(c: &mut Criterion) {
    let mut g = c.benchmark_group("scalar_product");
    for &n in SIZES {
        let xs = scalars(n);
        g.bench_with_input(BenchmarkId::new("serial", n), &n, |b, _| {
            b.iter(|| black_box(xs.iter().fold(S::one(), |a, x| a.mul(x))));
        });
        g.bench_with_input(BenchmarkId::new("parallel", n), &n, |b, _| {
            b.iter(|| black_box(xs.par_iter().cloned().reduce(S::one, |a, x| a.mul(&x))));
        });
    }
    g.finish();
}

/// point product reduce — `u_n_fold`, `h_n_fold` (`Π P_i`).
fn bench_point_product(c: &mut Criterion) {
    let mut g = c.benchmark_group("point_product");
    for &n in SIZES {
        let ps = elements(n);
        g.bench_with_input(BenchmarkId::new("serial", n), &n, |b, _| {
            b.iter(|| black_box(ps.iter().fold(E::one(), |a, p| a.mul(p))));
        });
        g.bench_with_input(BenchmarkId::new("parallel", n), &n, |b, _| {
            b.iter(|| black_box(ps.par_iter().cloned().reduce(E::one, |a, p| a.mul(&p))));
        });
    }
    g.finish();
}

/// hash-to-scalar loop — `e_n` (SHA3-512 + wide reduction per element).
fn bench_hash_to_scalar(c: &mut Criterion) {
    let seed = [0u8; 64];
    let tags: &[&[u8]; 2] = &[b"prefix", b"counter"];
    let mut g = c.benchmark_group("hash_to_scalar");
    for &n in SIZES {
        g.bench_with_input(BenchmarkId::new("serial", n), &n, |b, &n| {
            b.iter(|| {
                let v: Vec<S> = (0..n)
                    .map(|i| {
                        let ib = (i as u64).to_be_bytes();
                        let inputs: &[&[u8]] = &[&seed, &ib];
                        G::hash_to_scalar(inputs, tags).unwrap()
                    })
                    .collect();
                black_box(v)
            });
        });
        g.bench_with_input(BenchmarkId::new("parallel", n), &n, |b, &n| {
            b.iter(|| {
                let v: Vec<S> = (0..n)
                    .into_par_iter()
                    .map(|i| {
                        let ib = (i as u64).to_be_bytes();
                        let inputs: &[&[u8]] = &[&seed, &ib];
                        G::hash_to_scalar(inputs, tags).unwrap()
                    })
                    .collect();
                black_box(v)
            });
        });
    }
    g.finish();
}

/// point exp-map — `g_b_n` (`P_i^{s_i}`). Control: parallel should win clearly.
fn bench_point_exp_map(c: &mut Criterion) {
    let mut g = c.benchmark_group("point_exp_map");
    // 100k point-exps is seconds; cap this control at the smaller sizes.
    for &n in &[1_000usize, 10_000] {
        let (ps, ss) = (elements(n), scalars(n));
        g.bench_with_input(BenchmarkId::new("serial", n), &n, |b, _| {
            b.iter(|| {
                let out: Vec<E> = ps.iter().zip(ss.iter()).map(|(p, s)| p.exp(s)).collect();
                black_box(out)
            });
        });
        g.bench_with_input(BenchmarkId::new("parallel", n), &n, |b, _| {
            b.iter(|| {
                let out: Vec<E> = ps
                    .par_iter()
                    .zip(ss.par_iter())
                    .map(|(p, s)| p.exp(s))
                    .collect();
                black_box(out)
            });
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_scalar_rng,
    bench_scalar_mul_add,
    bench_scalar_inner_product,
    bench_scalar_product,
    bench_point_product,
    bench_hash_to_scalar,
    bench_point_exp_map,
);
criterion_main!(benches);
