// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: Apache-2.0

//! Stage breakdown behind the `profile` feature.
//!
//! Wall-clock accumulators for the coarse cost categories of the crypto stages
//! — multi-exponentiation (constant- and variable-time), fixed-base batches,
//! per-element exponentiation, transcript serialization, hashing, generator
//! derivation — collected at the **outer, sequential call sites** of each
//! stage, so that a category's time is wall-clock a caller would observe and
//! never a sum over rayon workers. Nested attribution is avoided by
//! construction: no timed region contains another.
//!
//! A benchmark (`examples/targets.rs`) calls [`reset`] before a stage and
//! [`snapshot`] after it; the difference between the stage's own wall-clock
//! and the categories' sum is the unattributed remainder. With the feature
//! off, [`timed`] is the identity and [`snapshot`] is empty: no cost, and the
//! production build carries no timers.
//!
//! The crypto code carries **no call sites** at present: the instrumentation
//! that produced PERFORMANCE.md's "Where the time goes" (2026-09-24) was
//! removed once measured, so that it does not clutter the implementation.
//! To measure again, wrap the outer, sequential call sites in
//! `timed(Category::…, || …)` — the per-site audit in PERFORMANCE.md
//! (Constraints) lists them — build with `--features profile`, and run
//! `targets`.

/// A coarse cost category of the crypto stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// Constant-time multi-exponentiations over secret scalars: the prover's
    /// `A′` and `F′`.
    MsmConstTime,
    /// Variable-time multi-exponentiations over public data: the shuffle
    /// verifier's, decryption's `a`/`b` and Lagrange combination, the batched
    /// Naor-Yung check.
    MsmVarTime,
    /// Fixed-base batches (`exp_many`): the bridging chain and `B′`.
    FixedBase,
    /// Per-element exponentiations that are not batched: the prover's
    /// permutation commitments and re-encryption legs, the decryption factors.
    ElementExp,
    /// Transcript serialization: the element and ciphertext lists encoded for
    /// the challenge hashes.
    TranscriptSer,
    /// Hashing: transcript seeds and the per-index or per-ballot challenge
    /// derivations (including the per-ballot serialization they contain).
    Hash,
    /// Independent generator derivation.
    Generators,
}

/// Every category, in reporting order.
pub const CATEGORIES: [Category; 7] = [
    Category::MsmConstTime,
    Category::MsmVarTime,
    Category::FixedBase,
    Category::ElementExp,
    Category::TranscriptSer,
    Category::Hash,
    Category::Generators,
];

impl Category {
    /// A short fixed label for reports.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Category::MsmConstTime => "msm (constant-time)",
            Category::MsmVarTime => "msm (variable-time)",
            Category::FixedBase => "fixed-base batches",
            Category::ElementExp => "per-element exps",
            Category::TranscriptSer => "transcript ser",
            Category::Hash => "hashing",
            Category::Generators => "generators",
        }
    }

    /// The category's position in [`CATEGORIES`] and in the accumulators.
    fn index(self) -> usize {
        match self {
            Category::MsmConstTime => 0,
            Category::MsmVarTime => 1,
            Category::FixedBase => 2,
            Category::ElementExp => 3,
            Category::TranscriptSer => 4,
            Category::Hash => 5,
            Category::Generators => 6,
        }
    }
}

/// One category's accumulated time and call count since the last [`reset`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// The category.
    pub category: Category,
    /// Accumulated wall-clock, in milliseconds.
    pub millis: f64,
    /// Number of timed regions accumulated.
    pub calls: u64,
}

/// The accumulators, present only with the `profile` feature.
#[cfg(feature = "profile")]
mod backend {
    use super::{CATEGORIES, Category, Sample};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

    /// Accumulated nanoseconds per category, indexed by [`Category::index`].
    static NANOS: [AtomicU64; 7] = [const { AtomicU64::new(0) }; 7];
    /// Timed regions per category, indexed by [`Category::index`].
    static CALLS: [AtomicU64; 7] = [const { AtomicU64::new(0) }; 7];

    /// The accumulator pair of a category.
    fn slot(category: Category) -> (&'static AtomicU64, &'static AtomicU64) {
        let i = category.index();
        (
            NANOS.get(i).expect("seven categories, seven slots"),
            CALLS.get(i).expect("seven categories, seven slots"),
        )
    }

    /// Run `f` and add its wall-clock to `category`.
    pub(super) fn timed<R>(category: Category, f: impl FnOnce() -> R) -> R {
        let start = Instant::now();
        let result = f();
        let nanos = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let (total, calls) = slot(category);
        total.fetch_add(nanos, Ordering::Relaxed);
        calls.fetch_add(1, Ordering::Relaxed);
        result
    }

    /// Zero every accumulator.
    pub(super) fn reset() {
        for (nanos, calls) in NANOS.iter().zip(CALLS.iter()) {
            nanos.store(0, Ordering::Relaxed);
            calls.store(0, Ordering::Relaxed);
        }
    }

    /// Read every accumulator, in [`CATEGORIES`] order.
    pub(super) fn snapshot() -> Vec<Sample> {
        CATEGORIES
            .iter()
            .map(|&category| {
                let (total, calls) = slot(category);
                // A reporting conversion: nanoseconds well below 2^53 for any
                // run this tool measures.
                #[allow(clippy::cast_precision_loss)]
                let millis = total.load(Ordering::Relaxed) as f64 / 1_000_000.0;
                Sample {
                    category,
                    millis,
                    calls: calls.load(Ordering::Relaxed),
                }
            })
            .collect()
    }
}

/// The no-op backend of the production build.
#[cfg(not(feature = "profile"))]
mod backend {
    use super::{Category, Sample};

    /// Run `f`; nothing is recorded.
    #[inline]
    pub(super) fn timed<R>(_category: Category, f: impl FnOnce() -> R) -> R {
        f()
    }

    /// Nothing to zero.
    pub(super) fn reset() {}

    /// No samples.
    pub(super) fn snapshot() -> Vec<Sample> {
        Vec::new()
    }
}

/// Run `f`, attributing its wall-clock to `category`.
///
/// With the `profile` feature off this is the identity.
#[inline]
pub fn timed<R>(category: Category, f: impl FnOnce() -> R) -> R {
    backend::timed(category, f)
}

/// Zero every accumulator. Call before the stage to attribute.
pub fn reset() {
    backend::reset();
}

/// The accumulated samples since the last [`reset`], in [`CATEGORIES`] order;
/// empty with the `profile` feature off.
#[must_use]
pub fn snapshot() -> Vec<Sample> {
    backend::snapshot()
}
