// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! # Examples
//!
//! ```
//! use strand::context::{Ctx, Element, Exponent};
//! use strand::backend::ristretto::RistrettoCtx;
//! use strand::threshold;
//!
//! let ctx = RistrettoCtx;
//! let num_trustees = 3;
//! let threshold = 2;
//!
//! let (coefficients, commitments) =
//!     threshold::gen_coefficients(threshold, &ctx);
//!
//! let mut shares = vec![];
//! for i in 0..num_trustees {
//!     let share = threshold::compute_peer_share(i, threshold, &coefficients, &ctx);
//!     shares.push(share);
//! }
//!
//! // send the shares to the other trustees..
//!
//! // trustees verify their received share
//! for i in 0..num_trustees {
//!     let vkf = threshold::verification_key_factor(
//!         &commitments,
//!         threshold,
//!         i,
//!         &ctx,
//!     );
//!     let ok = threshold::verify_share(&shares[i], &vkf, &ctx);
//!     assert!(ok);
//! }
//! ```

use crate::context::{Ctx, Element, Exponent};
use crate::elgamal::Ciphertext;
use crate::util::StrandError;
use crate::zkp::{ChaumPedersen, Zkp};

// A degree n polynomial is determined by n + 1 points.
// A degree n polynomial has n + 1 coefficients.
// Thus, the number of coefficients = threshold.
/// Generates the coefficients of a polynomial with threshold number of
/// coefficients.
pub fn gen_coefficients<C: Ctx>(
    threshold: usize,
    ctx: &C,
) -> (Vec<C::X>, Vec<C::E>) {
    let mut rng = ctx.get_rng();
    let mut coefficients = vec![];
    let mut commitments = vec![];

    for _ in 0..threshold {
        let coeff = ctx.rnd_exp(&mut rng);
        let commitment = ctx.gmod_pow(&coeff);
        coefficients.push(coeff);
        commitments.push(commitment);
    }

    (coefficients, commitments)
}

/// Evaluates the polynomial at the given trustee position.
pub fn eval_poly<C: Ctx>(
    trustee: usize,
    threshold: usize,
    coefficients: &[C::X],
    ctx: &C,
) -> C::X {
    let mut sum = coefficients[0].clone();
    let mut power = C::X::mul_identity();
    let trustee_exp = ctx.exp_from_u64(trustee as u64);

    for coefficient in coefficients.iter().take(threshold).skip(1) {
        power = power.mul(&trustee_exp).modq(ctx);
        sum = sum.add(&coefficient.mul(&power).modq(ctx));
    }
    sum.modq(ctx)
}

/// Computes the share for the target trustee.
pub fn compute_peer_share<C: Ctx>(
    target_trustee: usize,
    threshold: usize,
    coefficients: &[C::X],
    ctx: &C,
) -> C::X {
    // i + 1: trustees start at 1
    eval_poly(target_trustee + 1, threshold, coefficients, ctx)
}

/// Verifies received share using trustee's vkf (computed from commitments)
pub fn verify_share<C: Ctx>(
    share: &C::X,
    v_key_factor: &C::E,
    ctx: &C,
) -> bool {
    ctx.gmod_pow(&share) == *v_key_factor
}

/// Computes the factor of the verification key for the receiving trustee using
/// the sender commitments. Note also that this value must equal g^share_ij from
/// i to j, this is checked when verifying received shares.
pub fn verification_key_factor<C: Ctx>(
    sender_commitments: &[C::E],
    threshold: usize,
    receiver_trustee: usize,
    ctx: &C,
) -> C::E {
    let mut accum = C::E::mul_identity();
    // Trustees start at 1
    let t = receiver_trustee + 1;
    for (i, commitment) in sender_commitments.iter().enumerate().take(threshold)
    {
        let power = t.pow(i as u32);
        let power_element = ctx.exp_from_u64(power as u64);

        accum = accum
            .mul(&ctx.emod_pow(commitment, &power_element))
            .modp(ctx);
    }

    accum
}

/// Computes the decryption factor and proof for the given ciphertext using the
/// trustee's share of the secret. The Zkp reference is passed as an
/// optimization to avoid object creation for each ciphertext.
#[inline(always)]
pub fn decryption_factor<C: Ctx>(
    c: &Ciphertext<C>,
    share: &C::X,
    v_key: &C::E,
    label: &[u8],
    zkp: &Zkp<C>,
    ctx: &C,
) -> Result<(C::E, ChaumPedersen<C>), StrandError> {
    let factor = ctx.emod_pow(&c.gr, share);
    let proof =
        zkp.decryption_proof(share, v_key, &factor, &c.mhr, &c.gr, label);
    // let ok = zkp.decryption_verify(&v_key, &factor, None, &c.mhr, &c.gr,
    // &proof, label); assert!(ok);
    Ok((factor, proof?))
}

/// Verify the decryption factor with respect to verification key. The Zkp
/// reference is passed as an optimization to avoid object creation for each
/// ciphertext
#[inline(always)]
pub fn verify_decryption_factor<C: Ctx>(
    c: &Ciphertext<C>,
    vk: &C::E,
    df: &C::E,
    proof: &ChaumPedersen<C>,
    label: &[u8],
    zkp: &Zkp<C>,
) -> Result<bool, StrandError> {
    zkp.verify_decryption(&vk, &df, &c.mhr, &c.gr, &proof, &label)
}

/// Computes the Lagrange coefficient for the given trustee.
pub fn lagrange<C: Ctx>(trustee: usize, present: &[usize], ctx: &C) -> C::X {
    let mut numerator = C::X::mul_identity();
    let mut denominator = C::X::mul_identity();
    let trustee_exp = ctx.exp_from_u64(trustee as u64);

    for p in present {
        if *p == trustee {
            continue;
        }
        let present_exp = ctx.exp_from_u64(*p as u64);

        let diff_exp = present_exp
            // We use sub_mod in case the result is negative
            .sub_mod(&trustee_exp, ctx)
            .modq(ctx);

        numerator = numerator.mul(&present_exp).modq(ctx);
        denominator = denominator.mul(&diff_exp).modq(ctx);
    }

    numerator.divq(&denominator, ctx)
}

#[cfg(any(test, feature = "wasmtest"))]
pub(crate) mod tests {
    use crate::context::{Ctx, Element, Exponent};
    use crate::elgamal::{Ciphertext, PublicKey};
    use crate::threshold;
    use crate::util::StrandError;

    use crate::zkp::{ChaumPedersen, Zkp};

    struct KeymakerT<C: Ctx> {
        num_trustees: usize,
        threshold: usize,
        coefficients: Vec<C::X>,
        commitments: Vec<C::E>,
        shares: Vec<C::X>,
        external_shares: Vec<C::X>,
        v_key_factors: Vec<C::E>,
        ctx: C,
    }

    impl<C: Ctx> KeymakerT<C> {
        fn gen(num_trustees: usize, threshold: usize, ctx: &C) -> KeymakerT<C> {
            let mut shares = vec![];
            let external_shares = vec![C::X::mul_identity(); num_trustees];
            let v_key_factors = vec![];

            let (coefficients, commitments) =
                threshold::gen_coefficients(threshold, ctx);

            for i in 0..num_trustees {
                // i + 1: trustees start at 1
                let share =
                    threshold::eval_poly(i + 1, threshold, &coefficients, ctx);
                shares.push(share);
            }

            KeymakerT {
                num_trustees,
                threshold,
                coefficients,
                commitments,
                shares,
                external_shares,
                v_key_factors,
                ctx: (*ctx).clone(),
            }
        }

        fn add_external_share(
            &mut self,
            source: usize,
            share: C::X,
            commitments: &[C::E],
            self_position: usize,
        ) -> bool {
            let vkf = threshold::verification_key_factor(
                commitments,
                self.threshold,
                self_position,
                &self.ctx,
            );
            let check = self.ctx.gmod_pow(&share);
            let ok = check == vkf;
            if ok {
                self.external_shares[source] = share;
                self.v_key_factors.push(vkf);
            }
            ok
        }

        fn secret_share(&self) -> C::X {
            let mut sum = C::X::add_identity();
            for i in 0..self.num_trustees {
                sum = sum.add(&self.external_shares[i]);
            }

            sum.modq(&self.ctx)
        }

        fn decryption_factor(
            &self,
            c: &Ciphertext<C>,
            label: &[u8],
        ) -> Result<(C::E, ChaumPedersen<C>), StrandError> {
            let zkp = Zkp::new(&self.ctx);
            let share = self.secret_share();
            let v_key = self.verification_key();
            let factor = self.ctx.emod_pow(&c.gr, &share);
            let proof = zkp.decryption_proof(
                &share, &v_key, &factor, &c.mhr, &c.gr, label,
            );
            // let ok = zkp.decryption_verify(&v_key, &factor, None, &c.mhr,
            // &c.gr, &proof, label); assert!(ok);
            Ok((factor, proof?))
        }

        fn verification_key(&self) -> C::E {
            assert_eq!(self.v_key_factors.len(), self.num_trustees);
            let mut key = C::E::mul_identity();
            for next in &self.v_key_factors {
                key = key.mul(next).modp(&self.ctx);
            }

            key
        }
    }

    pub(crate) fn test_threshold_generic<C: Ctx>(
        ctx: &C,
        num_trustees: usize,
        threshold: usize,
        data: C::P,
    ) {
        let zkp = Zkp::new(ctx);
        let mut pk = C::E::mul_identity();
        let mut trustees = Vec::new();
        for _ in 0..num_trustees {
            let trustee = KeymakerT::gen(num_trustees, threshold, ctx);
            pk = pk.mul(&trustee.commitments[0]).modp(ctx);
            trustees.push(trustee);
        }

        // distribute shares, including to ourselves
        for i in 0..num_trustees {
            for j in 0..num_trustees {
                let share = trustees[i].shares[j].clone();
                let commitments = trustees[i].commitments.clone();
                let ok =
                    trustees[j].add_external_share(i, share, &commitments, j);
                assert!(ok);
            }
        }

        let pk = PublicKey::from_element(&pk, ctx);
        let plaintext = ctx.encode(&data).unwrap();

        let c: Ciphertext<C> = pk.encrypt(&plaintext);
        // sanity check: all trustees present for decryption works
        let mut divider = C::E::mul_identity();
        for t in trustees.iter().take(num_trustees) {
            divider = divider
                .mul(&ctx.emod_pow(&c.gr, &t.coefficients[0]))
                .modp(ctx);
        }
        let decrypted = c.mhr.divp(&divider, ctx).modp(ctx);
        let decoded = ctx.decode(&decrypted);

        assert_eq!(data, decoded);

        let all = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let present = &all[0..threshold];
        let mut divider = C::E::mul_identity();

        for i in 0..present.len() {
            let v_key = trustees[present[i] - 1].verification_key();
            let (base, proof) =
                trustees[present[i] - 1].decryption_factor(&c, &[]).unwrap();
            let ok = zkp
                .verify_decryption(&v_key, &base, &c.mhr, &c.gr, &proof, &[])
                .unwrap();
            assert!(ok);

            let lagrange = threshold::lagrange(present[i], present, ctx);

            let next = ctx.emod_pow(&base, &lagrange);
            divider = divider.mul(&next).modp(ctx)
        }

        let decrypted = c.mhr.divp(&divider, ctx).modp(ctx);
        let decoded = ctx.decode(&decrypted);

        assert_eq!(data, decoded);

        let present = &all[0..threshold - 1];
        let mut divider = C::E::mul_identity();

        for i in 0..present.len() {
            let v_key = trustees[present[i] - 1].verification_key();
            let (base, proof) =
                trustees[present[i] - 1].decryption_factor(&c, &[]).unwrap();
            let ok = zkp
                .verify_decryption(&v_key, &base, &c.mhr, &c.gr, &proof, &[])
                .unwrap();
            assert!(ok);

            let lagrange = threshold::lagrange(present[i], present, ctx);

            let next = ctx.emod_pow(&base, &lagrange);
            divider = divider.mul(&next).modp(ctx)
        }

        let decrypted = c.mhr.divp(&divider, ctx).modp(ctx);
        let decoded = ctx.decode(&decrypted);

        assert_ne!(data, decoded);
    }
}
