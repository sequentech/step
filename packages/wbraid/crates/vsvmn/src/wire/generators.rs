// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Deriving "independent" generators from a random oracle (VMNV §6.8, §8.2).
//!
//! A proof of a shuffle needs an array `h = (h_0, ..., h_{N'-1})` of generators
//! for which nobody knows a non-trivial representation of the unit. Prover and
//! verifier derive them independently from the same seed, so an emitter has to
//! reproduce this exactly — the generators feed both the Pedersen commitments
//! and the batching seed.
//!
//! ```text
//! s = RO_seed(rho | leaf("generators"))
//! h = G_q.randomArray(N', PRG(s), n_r)
//! ```
//!
//! On an elliptic curve `randomArray` walks candidate x-coordinates and keeps
//! those that lie on the curve:
//!
//! 1. draw `ceil((n_p + n_r) / 8)` bytes from the PRG (45 for P-256 with
//!    `n_r = 100`);
//! 2. mask the leading byte down to exactly `n_p + n_r` bits;
//! 3. reduce modulo `p` to get a candidate `z`;
//! 4. keep `z` if `f(z) = z^3 + ax + b` is a quadratic residue, taking the
//!    **smaller** of the two square roots; otherwise discard and draw again.
//!
//! # The root choice is normalised one level up
//!
//! `ECqPGroup.sqrt` alone is *not* the whole rule. For `p = 3 mod 4` it returns
//! `a^((p+1)/4) mod p`, which is the larger of the two roots about half the
//! time; it is `randomElementArray` that then normalises with
//! `y' = p - y; if (y' < y) y = y'`, matching VMNV §6.8's "the square root that
//! is smallest when viewed as an integer in `[0, p-1]`".
//!
//! Reading only `sqrt` gets every x-coordinate right and half the
//! y-coordinates wrong — which is exactly how this was caught, since the
//! `bas.h` golden test showed all ten x values matching and five y values
//! inverted.

use num_bigint::BigUint;

use crate::wire::bytetree::ByteTree;
use crate::wire::crypto::{Hashfunction, Prg, RandomOracle};
use crate::wire::error::Result;

/// Domain string VMN uses when deriving generators
/// (`IndependentGeneratorsRO("generators", ...)`).
pub const GENERATORS_SID: &str = "generators";

/// Curve parameters needed for the walk.
pub struct CurveParams {
    /// Field prime `p`.
    pub p: BigUint,
    /// Curve coefficient `a` (reduced mod p, so `-3` is `p - 3`).
    pub a: BigUint,
    /// Curve coefficient `b`.
    pub b: BigUint,
    /// Bit length of `p`.
    pub p_bits: usize,
    /// Fixed byte width of an encoded coordinate.
    pub width: usize,
}

impl CurveParams {
    /// NIST P-256.
    pub fn p256() -> Self {
        let p = BigUint::parse_bytes(
            b"ffffffff00000001000000000000000000000000ffffffffffffffffffffffff",
            16,
        )
        .expect("valid P-256 prime");
        let b = BigUint::parse_bytes(
            b"5ac635d8aa3a93e7b3ebbd55769886bc651d06b0cc53b0f63bce3c3e27d2604b",
            16,
        )
        .expect("valid P-256 b");
        let a = &p - BigUint::from(3u32); // a = -3 mod p
        CurveParams {
            p,
            a,
            b,
            p_bits: 256,
            width: crate::wire::marshal::p256::WIDTH,
        }
    }

    /// `f(x) = x^3 + a*x + b mod p`.
    fn equation_f(&self, x: &BigUint) -> BigUint {
        let x2 = (x * x) % &self.p;
        let x3 = (x2 * x) % &self.p;
        (x3 + (&self.a * x) % &self.p + &self.b) % &self.p
    }

    /// Whether `v` is a non-zero quadratic residue mod p (Legendre symbol = 1).
    fn is_quadratic_residue(&self, v: &BigUint) -> bool {
        if v.count_ones() == 0 {
            return false;
        }
        let exponent = (&self.p - BigUint::from(1u32)) >> 1;
        v.modpow(&exponent, &self.p) == BigUint::from(1u32)
    }

    /// The **smaller** of the two square roots of `v` mod p.
    ///
    /// For `p = 3 mod 4` the root is `v^((p+1)/4) mod p`; the caller-side
    /// normalisation in `randomElementArray` then picks whichever of `y` and
    /// `p - y` is numerically smaller (see the module note).
    fn sqrt(&self, v: &BigUint) -> BigUint {
        let exponent = (&self.p + BigUint::from(1u32)) >> 2;
        let y = v.modpow(&exponent, &self.p);
        let y_neg = &self.p - &y;
        if y_neg < y {
            y_neg
        } else {
            y
        }
    }
}

/// The seed from which generators are derived (VMNV §8.2):
/// `s = RO_seed(rho | leaf("generators"))`.
pub fn generators_seed(hash: Hashfunction, rho: &[u8]) -> Vec<u8> {
    let sid_tree = ByteTree::leaf(GENERATORS_SID.as_bytes().to_vec());
    let mut input = Vec::new();
    input.extend_from_slice(rho);
    input.extend_from_slice(&sid_tree.to_bytes());
    RandomOracle::new(hash, hash.outlen_bits()).eval(&input)
}

/// Derive `count` independent generators as a byte-tree array of affine points.
///
/// `n_r` is the statistical-distance parameter (`statdist` in the protocol info
/// file), which widens each candidate before reduction mod `p`.
pub fn independent_generators(
    hash: Hashfunction,
    rho: &[u8],
    curve: &CurveParams,
    n_r: usize,
    count: usize,
) -> Result<ByteTree> {
    let seed = generators_seed(hash, rho);
    let prg = Prg::new(hash, &seed);

    let candidate_bits = curve.p_bits + n_r;
    let candidate_bytes = candidate_bits.div_ceil(8);
    let excess = candidate_bits % 8;

    let mut points = Vec::with_capacity(count);
    let mut consumed = 0usize;
    // Roughly half the candidates land on the curve; draw generously and grow
    // if needed. The PRG's output is a fixed stream, so a longer draw always
    // extends the shorter one and re-drawing never shifts what we already read.
    let mut buffer = prg.generate((4 * count + 16) * candidate_bytes);

    while points.len() < count {
        if consumed + candidate_bytes > buffer.len() {
            buffer = prg.generate(buffer.len() * 2);
            continue;
        }
        let mut raw = buffer[consumed..consumed + candidate_bytes].to_vec();
        consumed += candidate_bytes;

        // Mask the leading byte down to exactly `candidate_bits` bits.
        if excess != 0 {
            raw[0] &= 0xFFu8 >> (8 - excess);
        }

        let z = BigUint::from_bytes_be(&raw) % &curve.p;
        let fz = curve.equation_f(&z);
        if !curve.is_quadratic_residue(&fz) {
            continue;
        }
        let y = curve.sqrt(&fz);
        points.push(crate::wire::arithm::curve_point(
            &z.to_bytes_be(),
            &y.to_bytes_be(),
            curve.width,
        )?);
    }

    Ok(ByteTree::node(points))
}
