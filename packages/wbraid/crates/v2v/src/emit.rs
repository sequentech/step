// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Producing a Verificatum-format session with braid's cryptography.
//!
//! The counterpart of [`crate::session`], which reads one. Everything here runs
//! our own DKG, shuffle and threshold decryption and writes the result in the
//! layout `vmnv` expects, so that an independently written verifier can be
//! pointed at it.
//!
//! # This is a generator, not an exporter
//!
//! It produces a *synthetic* session: the ciphertexts are encryptions of random
//! group elements, and every party is played by this process. It does not take a
//! real braid session and convert it.
//!
//! The shuffle half of a real export is possible, and is essentially what
//! [`shuffling`] writes. The decryption half is not, and the obstacle is
//! structural rather than a missing feature: Verificatum's decryption transcript
//! is joint over *all* `k` parties' factors — the batching seed commits to every
//! party's factor array before any commitment is formed — so producing it needs
//! three rounds among the trustees. Braid's decryption protocol does not have
//! them.
//!
//! # Widths and party counts are compile-time
//!
//! `W`, `K` and `T` are const generic because vsc's ciphertext, DKG polynomial
//! and participant types are. A caller working from runtime values goes through
//! [`generate`], which dispatches to a fixed set of instantiations; the bounds
//! are listed there, and exceeding one is an error naming what to add.

use std::path::Path;

use anyhow::{anyhow, Context as _, Result};

use cryptography::context::{Context, P256Ctx};
use cryptography::cryptosystem::elgamal::{Ciphertext, KeyPair, PublicKey};
use cryptography::dkgd::dealer::{Dealer, VerifiableShare};
use cryptography::dkgd::recipient::{ParticipantPosition, Recipient};
use cryptography::groups::p256::element::P256Element;
use cryptography::groups::p256::scalar::P256Scalar;
use cryptography::traits::groups::{GroupElement, GroupScalar};
use cryptography::zkp::shuffle::{ShuffleProof, Shuffler};

use crate::challenges::VmnChallenges;
use crate::generators::vmn_generators;
use crate::proof_dir::{DecryptingParty, MixerStep, MixingProof, ShufflingProof};
use crate::wire::bytetree::ByteTree;
use crate::wire::crypto::{dec_challenge, dec_seed, global_prefix, Hashfunction, Prg};
use crate::wire::protinfo::ProtocolInfo;
use crate::{decrypt, encode};

/// What kind of session to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Shuffling only: a chain of mixers, no decryption. `vmnv -shuffle`.
    Shuffling,
    /// The whole session, ending in threshold decryption. `vmnv -mix`.
    Mixing,
}

/// Everything a session needs beyond the cryptography itself.
pub struct SessionSpec {
    /// The protocol parameters. These are also what must be written to the
    /// protocol info file the verifier is given: ρ commits to them, so a file
    /// describing anything else will not verify.
    pub info: ProtocolInfo,
    /// The auxiliary session identifier (`-auxsid`), `"default"` unless a
    /// caller has a reason otherwise.
    pub auxsid: String,
    /// How many ciphertexts to shuffle.
    pub ciphertexts: usize,
    /// Δ, the 1-based indices of the λ parties that take part. Parties not
    /// listed contribute Verificatum's placeholder decryption material.
    pub active: Vec<usize>,
}

impl SessionSpec {
    /// A P-256 session with the leading `threshold` parties taking part.
    #[must_use]
    pub fn p256(parties: usize, threshold: usize, width: usize, ciphertexts: usize) -> Self {
        SessionSpec {
            info: ProtocolInfo::p256("braid", parties, threshold, width),
            auxsid: "default".to_string(),
            ciphertexts,
            active: (1..=threshold).collect(),
        }
    }

    /// The global prefix ρ this session's transcripts are bound to.
    #[must_use]
    pub fn prefix(&self) -> Vec<u8> {
        global_prefix(Hashfunction::Sha256, &self.info.prefix_params(&self.auxsid))
    }

    /// Reject a specification that could not describe a real session, before
    /// any of it reaches the wire format.
    fn validate(&self, kind: Kind) -> Result<()> {
        if !self.info.is_consistent() {
            return Err(anyhow!(
                "k={}, lambda={}, width={} is not a possible session",
                self.info.parties,
                self.info.threshold,
                self.info.width
            ));
        }
        if self.ciphertexts == 0 {
            return Err(anyhow!("a session with no ciphertexts proves nothing"));
        }
        if kind == Kind::Mixing && self.active.len() != self.info.threshold {
            return Err(anyhow!(
                "exactly lambda = {} parties may decrypt, but {} are active",
                self.info.threshold,
                self.active.len()
            ));
        }
        if let Some(bad) = self.active.iter().find(|&&p| p < 1 || p > self.info.parties) {
            return Err(anyhow!(
                "active party {bad} is outside 1..={}",
                self.info.parties
            ));
        }
        let mut seen = self.active.clone();
        seen.sort_unstable();
        seen.dedup();
        if seen.len() != self.active.len() {
            return Err(anyhow!("a party may appear at most once in the active set"));
        }
        Ok(())
    }
}

/// Write a session of the given kind, dispatching the const generics from the
/// specification's runtime values.
///
/// Instantiated for width 1–3, `k` 1–4 and every threshold within it. Those
/// bounds exist only because each combination is a separate monomorphisation;
/// widening them is a matter of adding arms, which the error says.
///
/// # Errors
///
/// If the specification is impossible, if no instantiation covers its shape, or
/// if the directory cannot be written.
pub fn generate(spec: &SessionSpec, kind: Kind, dir: &Path) -> Result<()> {
    spec.validate(kind)?;
    match (kind, spec.info.width) {
        (Kind::Shuffling, 1) => shuffling::<1>(spec, dir),
        (Kind::Shuffling, 2) => shuffling::<2>(spec, dir),
        (Kind::Shuffling, 3) => shuffling::<3>(spec, dir),
        (Kind::Mixing, 1) => mixing_arms::<1>(spec, dir),
        (Kind::Mixing, 2) => mixing_arms::<2>(spec, dir),
        (Kind::Mixing, 3) => mixing_arms::<3>(spec, dir),
        (_, w) => Err(anyhow!(
            "no emitter instantiation for width {w}; add one to emit::generate"
        )),
    }
}

/// The `(k, λ)` half of [`generate`]'s dispatch, once the width is fixed.
fn mixing_arms<const W: usize>(spec: &SessionSpec, dir: &Path) -> Result<()> {
    match (spec.info.parties, spec.info.threshold) {
        (1, 1) => mixing::<W, 1, 1>(spec, dir),
        (2, 1) => mixing::<W, 2, 1>(spec, dir),
        (2, 2) => mixing::<W, 2, 2>(spec, dir),
        (3, 1) => mixing::<W, 3, 1>(spec, dir),
        (3, 2) => mixing::<W, 3, 2>(spec, dir),
        (3, 3) => mixing::<W, 3, 3>(spec, dir),
        (4, 1) => mixing::<W, 4, 1>(spec, dir),
        (4, 2) => mixing::<W, 4, 2>(spec, dir),
        (4, 3) => mixing::<W, 4, 3>(spec, dir),
        (4, 4) => mixing::<W, 4, 4>(spec, dir),
        (k, t) => Err(anyhow!(
            "no emitter instantiation for k={k}, lambda={t}; add one to emit::mixing_arms"
        )),
    }
}

/// A chain of λ mixers with no decryption phase — `type = shuffling`.
///
/// The independent generators are a **session-level** value derived once from
/// the prefix; every mixer shares them. Only the per-mixer statement differs,
/// since each batching seed commits to that mixer's own permutation commitment
/// and its input/output pair.
///
/// # Errors
///
/// If the shuffle or the encoding fails, or the directory cannot be written.
pub fn shuffling<const W: usize>(spec: &SessionSpec, dir: &Path) -> Result<()> {
    shuffling_with_prefix::<W>(spec, &spec.prefix(), dir)
}

/// As [`shuffling`], with ρ supplied rather than derived.
///
/// Exists for callers that must control the prefix — checking that a verifier
/// rejects a proof built against the wrong one, for instance. Anything writing a
/// session to be verified wants [`shuffling`].
///
/// # Errors
///
/// As [`shuffling`].
pub fn shuffling_with_prefix<const W: usize>(
    spec: &SessionSpec,
    rho: &[u8],
    dir: &Path,
) -> Result<()> {
    width_matches::<W>(spec)?;
    let mixers = spec.active.len().max(1);
    let n = spec.ciphertexts;

    let keypair: KeyPair<P256Ctx> = KeyPair::generate();
    let input = encrypt_random::<W>(&keypair.pkey, n);

    let generators = vmn_generators(Hashfunction::Sha256, rho, spec.info.n_r as usize, n)
        .context("deriving independent generators")?;
    let shuffler = Shuffler::<P256Ctx, W>::new(generators, keypair.pkey.clone());

    let mut current = input.clone();
    let mut outputs = Vec::with_capacity(mixers);
    let mut proofs = Vec::with_capacity(mixers);
    for _ in 0..mixers {
        let challenges = challenges(spec, rho);
        let (output, proof) = shuffler
            .shuffle_with(&current, &[], &challenges)
            .map_err(|e| anyhow!("shuffling: {e:?}"))?;
        current = output.clone();
        outputs.push(output);
        proofs.push(proof);
    }

    let gamma = arbitrary_polynomial(&keypair.pkey.y, spec.info.threshold);
    let steps = steps(&outputs, &proofs);

    ShufflingProof::<W> {
        version: &spec.info.version,
        auxsid: &spec.auxsid,
        width: W,
        threshold: spec.info.threshold,
        public_key: &keypair.pkey.y,
        input: &input,
        mixers: &steps,
        polynomial_in_exponent: Some(&gamma),
    }
    .write(dir)
    .context("writing the shuffling proof")
}

/// A complete `type = mixing` session: a real DKG, a chain of λ shuffles, and
/// threshold decryption with the batched proof.
///
/// `K` is the party count `k` and `T` the threshold λ, both of which must match
/// the protocol info file. Parties outside `spec.active` contribute the
/// all-identity factor array and the identity commitment and zero reply that
/// Verificatum records for an absent contribution — which they must, since the
/// verifier reads a file for every party and the batching seed commits to all
/// of them.
///
/// The chain runs λ mixers, since `λ_a ≥ λ` and braid mixes with exactly its
/// selected trustees.
///
/// # Errors
///
/// If the DKG, shuffle or encoding fails, or the directory cannot be written.
pub fn mixing<const W: usize, const K: usize, const T: usize>(
    spec: &SessionSpec,
    dir: &Path,
) -> Result<()> {
    width_matches::<W>(spec)?;
    if (K, T) != (spec.info.parties, spec.info.threshold) {
        return Err(anyhow!(
            "emitting at k={K}, lambda={T} a session that declares k={}, lambda={}",
            spec.info.parties,
            spec.info.threshold
        ));
    }
    let delta = &spec.active;
    let n = spec.ciphertexts;
    let rho = spec.prefix();

    // --- the distributed key generation -----------------------------------
    // VMN has no counterpart to the checking-value proofs; they exist here
    // because `from_shares` verifies complete dealings, and are not emitted.
    const CORPUS_DKG_CTX: &[u8] = b"v2v corpus dkg";
    let dealers: Vec<Dealer<P256Ctx, T, K>> = (0..K).map(|_| Dealer::generate()).collect();
    let dealt: Vec<_> = dealers
        .iter()
        .map(|d| d.get_verifiable_shares(CORPUS_DKG_CTX))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow!("dealing failed: {e:?}"))?;

    let gamma = decrypt::polynomial_in_exponent(
        &dealt
            .iter()
            .map(|s| {
                s.checking_values
                    .iter()
                    .map(|cv| cv.value)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )
    .context("deriving the polynomial in the exponent")?;

    // Each party verifies every dealer's contribution and keeps its share.
    let mut secrets = Vec::with_capacity(K);
    let mut joint_key = None;
    for party in 1..=K {
        let shares: [VerifiableShare<P256Ctx, T>; K] = std::array::from_fn(|d| {
            VerifiableShare::new(
                dealt[d].shares[party - 1].clone(),
                dealt[d].checking_values.clone(),
            )
        });
        let (recipient, joint_pk, _vks) = Recipient::<P256Ctx, T, K>::from_shares(
            ParticipantPosition::from_usize(party),
            &shares,
            CORPUS_DKG_CTX,
        )
        .map_err(|e| anyhow!("share verification failed for party {party}: {e:?}"))?;
        joint_key = Some(joint_pk.y);
        secrets.push(*recipient.get_secret_share());
    }
    let y = joint_key.ok_or_else(|| anyhow!("a session needs at least one party"))?;
    if !gamma[0].equals(&y) {
        return Err(anyhow!(
            "Gamma_0 is not the joint public key; the DKG and the polynomial disagree"
        ));
    }

    // --- the shuffle chain --------------------------------------------------
    let pk = PublicKey::<P256Ctx>::new(y);
    let input = encrypt_random::<W>(&pk, n);

    let generators = vmn_generators(Hashfunction::Sha256, &rho, spec.info.n_r as usize, n)
        .context("deriving independent generators")?;
    let shuffler = Shuffler::<P256Ctx, W>::new(generators, pk.clone());

    let mut current = input.clone();
    let mut outputs = Vec::with_capacity(T);
    let mut shuffle_proofs = Vec::with_capacity(T);
    for _ in 0..T {
        let challenges = challenges(spec, &rho);
        let (output, proof) = shuffler
            .shuffle_with(&current, &[], &challenges)
            .map_err(|e| anyhow!("shuffling: {e:?}"))?;
        current = output.clone();
        outputs.push(output);
        shuffle_proofs.push(proof);
    }
    let mixed = outputs
        .last()
        .ok_or_else(|| anyhow!("a session needs at least one mixer"))?
        .clone();

    // --- decryption factors, in Verificatum's convention --------------------
    // A participant scales its share by 1/alpha once: the factors use the
    // negation of that scalar and the proof reply uses it directly.
    let inv_alpha = decrypt::inverse_alpha(K).context("deriving 1/alpha")?;
    let u: Vec<[P256Element; W]> = mixed.iter().map(|c| c.0[0]).collect();

    let scaled: Vec<Option<P256Scalar>> = (1..=K)
        .map(|party| delta.contains(&party).then(|| secrets[party - 1].mul(&inv_alpha)))
        .collect();
    let factors: Vec<Vec<[P256Element; W]>> = scaled
        .iter()
        .map(|z| match z {
            Some(z) => {
                let exponent = z.neg();
                u.iter()
                    .map(|ui| std::array::from_fn(|w| ui[w].exp(&exponent)))
                    .collect()
            }
            None => decrypt::inactive_factors::<W>(n),
        })
        .collect();

    // --- the batched proof transcript ---------------------------------------
    let factor_trees: Vec<ByteTree> = factors
        .iter()
        .map(|f| encode::component_array_to_tree(f))
        .collect::<Result<_>>()
        .context("encoding the decryption factors")?;
    let seed = dec_seed(
        Hashfunction::Sha256,
        &rho,
        &encode::element_to_tree(&P256Element::generator()).context("encoding the generator")?,
        &encode::ciphertexts_to_tree(&mixed).context("encoding the mixed ciphertexts")?,
        &encode::elements_to_tree(&gamma).context("encoding the polynomial")?,
        &factor_trees,
    );

    // One n_e-bit batching exponent per ciphertext, as in the shuffle.
    let component = (spec.info.n_e as usize).div_ceil(8);
    let stream = Prg::new(Hashfunction::Sha256, &seed).generate(component * n);
    let e: Vec<P256Scalar> = stream.chunks(component).map(scalar_from).collect();
    let a = decrypt::batch(&u, &e).context("batching the first components")?;

    // Commitments do not depend on the challenge, so they are fixed first and
    // then hashed into it -- including the non-participants', which is why
    // their placeholder values are not free to choose.
    let mut rng = P256Ctx::get_rng();
    let zero = P256Scalar::zero();
    let randomizers: Vec<Option<P256Scalar>> = scaled
        .iter()
        .map(|z| z.as_ref().map(|_| P256Scalar::random(&mut rng)))
        .collect();
    let commitments: Vec<_> = randomizers
        .iter()
        .map(|r| match r {
            Some(r) => decrypt::prove_decryption::<W>(&zero, &a, &zero, r),
            None => decrypt::inactive_proof::<W>(),
        })
        .collect();
    let commitment_trees: Vec<ByteTree> = commitments
        .iter()
        .map(|c| -> Result<ByteTree> {
            Ok(ByteTree::node(vec![
                encode::element_to_tree(&c.y_prime)?,
                encode::elements_to_tree(&c.b_prime)?,
            ]))
        })
        .collect::<Result<_>>()
        .context("encoding the decryption commitments")?;

    let v = scalar_from(&dec_challenge(
        Hashfunction::Sha256,
        spec.info.n_v as usize,
        &rho,
        &seed,
        &commitment_trees,
    ));
    let proofs: Vec<_> = (0..K)
        .map(|l| match (&scaled[l], &randomizers[l]) {
            (Some(z), Some(r)) => decrypt::prove_decryption::<W>(z, &a, &v, r),
            _ => decrypt::inactive_proof::<W>(),
        })
        .collect();

    // --- the plaintexts ------------------------------------------------------
    let alpha_c: Vec<P256Scalar> = crate::wire::lagrange::p256_modified_lagrange_coefficients(delta, K)
        .into_iter()
        .map(|(negative, magnitude)| {
            let s = P256Scalar::from_bytes_reduced(&magnitude);
            if negative {
                s.neg()
            } else {
                s
            }
        })
        .collect();
    let plaintexts: Vec<[P256Element; W]> = (0..n)
        .map(|i| {
            let mut combined: [P256Element; W] = std::array::from_fn(|_| P256Element::one());
            for (position, &party) in delta.iter().enumerate() {
                for w in 0..W {
                    let contribution = factors[party - 1][i][w].exp(&alpha_c[position]);
                    combined[w] = combined[w].mul(&contribution);
                }
            }
            std::array::from_fn(|w| mixed[i].0[1][w].mul(&combined[w]))
        })
        .collect();

    // --- write it out --------------------------------------------------------
    let steps = steps(&outputs, &shuffle_proofs);
    let parties: Vec<DecryptingParty<W>> = (0..K)
        .map(|l| DecryptingParty {
            factors: &factors[l],
            proof: &proofs[l],
            participated: delta.contains(&(l + 1)),
        })
        .collect();

    MixingProof::<W> {
        shuffle: ShufflingProof {
            version: &spec.info.version,
            auxsid: &spec.auxsid,
            width: W,
            threshold: T,
            public_key: &y,
            input: &input,
            mixers: &steps,
            polynomial_in_exponent: Some(&gamma),
        },
        plaintexts: &plaintexts,
        parties: &parties,
    }
    .write(dir)
    .context("writing the mixing proof")
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

fn width_matches<const W: usize>(spec: &SessionSpec) -> Result<()> {
    if W == spec.info.width {
        return Ok(());
    }
    Err(anyhow!(
        "emitting at width {W} a session that declares width {}",
        spec.info.width
    ))
}

fn challenges(spec: &SessionSpec, rho: &[u8]) -> VmnChallenges {
    VmnChallenges::new(
        Hashfunction::Sha256,
        rho.to_vec(),
        spec.info.n_e as usize,
        spec.info.n_v as usize,
        spec.info.width,
    )
}

fn encrypt_random<const W: usize>(pk: &PublicKey<P256Ctx>, n: usize) -> Vec<Ciphertext<P256Ctx, W>> {
    (0..n)
        .map(|_| {
            let m: [<P256Ctx as Context>::Element; W] =
                std::array::from_fn(|_| P256Ctx::random_element());
            pk.encrypt(&m)
        })
        .collect()
}

fn steps<'a, const W: usize>(
    outputs: &'a [Vec<Ciphertext<P256Ctx, W>>],
    proofs: &'a [ShuffleProof<P256Ctx, W>],
) -> Vec<MixerStep<'a, W>> {
    outputs
        .iter()
        .zip(proofs.iter())
        .map(|(output, proof)| MixerStep { output, proof })
        .collect()
}

/// A polynomial in the exponent `Γ = (Γ_0, ..., Γ_{λ-1})` with `Γ_0 = y` and
/// arbitrary higher coefficients.
///
/// `vmnv` does not read this for a shuffling proof, but VMNV §9.3 step 5 and
/// §9.1 both say a proof directory contains it, and VMN's own prover writes it
/// even for shuffling sessions. Emitting it keeps braid's proofs acceptable to a
/// verifier written strictly to the specification, rather than only to one that
/// shares `vmnv`'s leniency.
///
/// Every element of a prime-order group is `g^γ` for some `γ`, so this is a
/// well-formed degree `λ-1` polynomial whose constant term is the real secret; a
/// shuffling session never decrypts, so the coefficients above it are unused.
fn arbitrary_polynomial(y: &P256Element, threshold: usize) -> Vec<P256Element> {
    let mut gamma = Vec::with_capacity(threshold);
    gamma.push(*y);
    for _ in 1..threshold {
        gamma.push(P256Ctx::random_element());
    }
    gamma
}

/// Interpret a big-endian byte string as a scalar, reducing modulo the group
/// order.
///
/// Verificatum reads these as unbounded non-negative integers and exponentiates
/// by them, which is the same thing in a group of prime order.
fn scalar_from(bytes: &[u8]) -> P256Scalar {
    let mut wide = [0u8; 32];
    let start = 32 - bytes.len().min(32);
    wide[start..].copy_from_slice(&bytes[bytes.len().saturating_sub(32)..]);
    P256Scalar::from_bytes_reduced(&wide)
}
