// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Writing a Verificatum proof directory (VMNV §9.1).
//!
//! A proof is a directory, not a file: a handful of ASCII parameters and byte
//! trees at the root, plus a `proofs/` subdirectory holding the intermediate
//! values and the zero-knowledge proofs relating them. This module writes the
//! **shuffling** form — a re-encryption and permutation with no decryption — so
//! that `vmnv -shuffle` can check a shuffle braid performed.
//!
//! ```text
//! <dir>/version type auxsid width
//!       FullPublicKey.bt  Ciphertexts.bt  ShuffledCiphertexts.bt
//!       proofs/ activethreshold  PolynomialInExponent.bt
//!               Ciphertexts01.bt  PermutationCommitment01.bt
//!               PoSCommitment01.bt  PoSReply01.bt
//! ```
//!
//! Two details that are easy to miss:
//!
//! - `ShuffledCiphertexts.bt` (the session output `L_λa`) and the last mixer's
//!   `Ciphertexts<l>.bt` are the same list, but both files are required.
//! - VMNV §9.3 step 5 reads the keys, which Algorithm 24 splits into two: the
//!   joint public key `pk`, and the polynomial in the exponent `Γ` with the
//!   check `Γ_0 ≠ y ⇒ reject`. `vmnv` does the first unconditionally and the
//!   second only when verifying decryption. Verified against the
//!   implementation: deleting `FullPublicKey.bt` fails a `-shuffle` run
//!   outright, while deleting `PolynomialInExponent.bt` leaves it at exit 0.
//!
//!   `pk` is genuinely needed by a shuffling proof — Algorithm 25 takes it as an
//!   input, and it appears in the fifth verification equation. `Γ` is not among
//!   Algorithm 25's inputs, which is why skipping it is sound for `vmnv`. But a
//!   verifier written to the specification will still read it *and* check it
//!   against `pk`, so an emitter should supply it: **if present it must satisfy
//!   `Γ_0 = y`**, and it must have `λ` entries.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};

use cryptography::context::P256Ctx;
use cryptography::cryptosystem::elgamal::Ciphertext;
use cryptography::groups::p256::element::P256Element;
use cryptography::zkp::shuffle::ShuffleProof;

use crate::wire::bytetree::ByteTree;

use crate::decrypt::BatchedDecryptionProof;
use crate::{challenges::commitments_to_tree, encode};

/// One mixer's contribution to the chain: the list it produced and the proof
/// relating it to the previous one.
pub struct MixerStep<'a, const W: usize> {
    /// This mixer's output `L_l`.
    pub output: &'a [Ciphertext<P256Ctx, W>],
    /// Proof that `L_l` is a re-encryption and permutation of `L_{l-1}`.
    pub proof: &'a ShuffleProof<P256Ctx, W>,
}

/// Everything needed to write a shuffling proof.
pub struct ShufflingProof<'a, const W: usize> {
    /// VMN version the proof claims conformance with; must match the verifier.
    pub version: &'a str,
    /// Auxiliary session identifier (`"default"` unless set otherwise).
    pub auxsid: &'a str,
    /// Ciphertext width omega.
    pub width: usize,
    /// The joint public key `y` (the generator half is implied).
    pub public_key: &'a P256Element,
    /// The session threshold `λ`, from the protocol info file's `<thres>`.
    ///
    /// Used to check what the directory claims against what the session
    /// declares: the polynomial must have exactly `λ` entries, and there must be
    /// at least `λ` mixers, since `λ_a ≥ λ` by construction.
    pub threshold: usize,
    /// Input ciphertexts `L_0`.
    pub input: &'a [Ciphertext<P256Ctx, W>],
    /// The mixers in order, each consuming the previous list. Must be non-empty;
    /// its length is the active threshold `λ_a`, and the last output is the
    /// session result `L_λa`.
    pub mixers: &'a [MixerStep<'a, W>],
    /// The polynomial in the exponent `Γ`, if known.
    ///
    /// `vmnv -shuffle` does not read it, but a verifier following VMNV §9.3
    /// step 5 does, and checks it against the public key, so supplying it is
    /// what makes a proof directory acceptable to more than just `vmnv`.
    ///
    /// **If supplied it must satisfy `Γ_0 = y`** (Algorithm 24 rejects
    /// otherwise) and hold `λ` entries for the session's threshold. Omitting it
    /// is safer than supplying a wrong one.
    pub polynomial_in_exponent: Option<&'a [P256Element]>,
}

impl<const W: usize> ShufflingProof<'_, W> {
    /// Write the proof directory at `dir`, creating it if absent.
    pub fn write(&self, dir: &Path) -> Result<()> {
        if self.mixers.is_empty() {
            return Err(anyhow!("a shuffling proof needs at least one mixer"));
        }
        // The active threshold is the number of mixers, and lambda_a >= lambda
        // by construction, so fewer mixers than the threshold describes a
        // session that can never satisfy it.
        if self.mixers.len() < self.threshold {
            return Err(anyhow!(
                "{} mixers cannot meet a threshold of {}",
                self.mixers.len(),
                self.threshold
            ));
        }

        // Algorithm 24 reads Gamma = (Gamma_0, ..., Gamma_{lambda-1}) and
        // rejects if that read fails or if Gamma_0 != y. Both halves are checked
        // here rather than emitting a directory a spec-following verifier would
        // refuse for a reason unrelated to the shuffle -- `vmnv -shuffle` skips
        // this check entirely, so nothing downstream would catch it for us.
        if let Some(gamma) = self.polynomial_in_exponent {
            if gamma.len() != self.threshold {
                return Err(anyhow!(
                    "the polynomial in the exponent has {} entries, expected {} \
                     (one per threshold)",
                    gamma.len(),
                    self.threshold
                ));
            }
            if gamma[0] != *self.public_key {
                return Err(anyhow!(
                    "polynomial in the exponent is inconsistent with the public key: \
                     Gamma_0 != y"
                ));
            }
        }
        let proofs = dir.join("proofs");
        fs::create_dir_all(&proofs)?;

        // --- root: parameters -------------------------------------------
        write_ascii(&dir.join("version"), self.version)?;
        write_ascii(&dir.join("type"), "shuffling")?;
        write_ascii(&dir.join("auxsid"), self.auxsid)?;
        write_ascii(&dir.join("width"), &self.width.to_string())?;

        // --- root: statement --------------------------------------------
        write_tree(
            &dir.join("FullPublicKey.bt"),
            &encode::public_key_to_tree(self.public_key)?,
        )?;
        write_tree(
            &dir.join("Ciphertexts.bt"),
            &encode::ciphertexts_to_tree(self.input)?,
        )?;
        // The session output is the final mixer's list.
        let last = self.mixers.last().expect("mixers is non-empty");
        write_tree(
            &dir.join("ShuffledCiphertexts.bt"),
            &encode::ciphertexts_to_tree(last.output)?,
        )?;

        // --- proofs: session-level values --------------------------------
        write_ascii(
            &proofs.join("activethreshold"),
            &self.mixers.len().to_string(),
        )?;
        if let Some(gamma) = self.polynomial_in_exponent {
            write_tree(
                &proofs.join("PolynomialInExponent.bt"),
                &encode::elements_to_tree(gamma)?,
            )?;
        }

        // --- proofs: one set of files per mixer --------------------------
        for (index, mixer) in self.mixers.iter().enumerate() {
            // File suffixes are 1-based and zero-padded to two digits (§9.1).
            let l = index + 1;
            write_tree(
                &proofs.join(format!("Ciphertexts{l:02}.bt")),
                &encode::ciphertexts_to_tree(mixer.output)?,
            )?;

            let commitments = &mixer.proof.commitments;
            write_tree(
                &proofs.join(format!("PermutationCommitment{l:02}.bt")),
                &encode::elements_to_tree(commitments.u_n())?,
            )?;
            write_tree(
                &proofs.join(format!("PoSCommitment{l:02}.bt")),
                &commitments_to_tree(commitments)
                    .map_err(|e| anyhow!("failed to encode proof commitment: {e:?}"))?,
            )?;
            write_tree(
                &proofs.join(format!("PoSReply{l:02}.bt")),
                &responses_to_tree(mixer.proof)?,
            )?;
        }

        Ok(())
    }
}

/// `sigma^pos = node(k_A, k_B, k_C, k_D, k_E, k_F)` (VMNV §8.3).
fn responses_to_tree<const W: usize>(proof: &ShuffleProof<P256Ctx, W>) -> Result<ByteTree> {
    let r = &proof.responses;
    Ok(ByteTree::node(vec![
        encode::scalar_to_tree(&r.k_a)?,
        encode::scalars_to_tree(&r.k_b_n)?,
        encode::scalar_to_tree(&r.k_c)?,
        encode::scalar_to_tree(&r.k_d)?,
        encode::scalars_to_tree(&r.k_e_n)?,
        encode::scalars_to_tree(&r.k_f)?,
    ]))
}

fn write_ascii(path: &Path, value: &str) -> Result<()> {
    fs::write(path, value.as_bytes())?;
    Ok(())
}

fn write_tree(path: &Path, tree: &ByteTree) -> Result<()> {
    fs::write(path, tree.to_bytes())?;
    Ok(())
}

/// One party's decryption contribution.
///
/// Every party in `1..=k` needs one, including those that took no part: the
/// verifier reads factors, commitments and replies over the full range, and all
/// of them are hashed into the decryption challenge. A non-participant supplies
/// an all-identity factor array ([`crate::decrypt::inactive_factors`]) and is
/// marked `participated = false`, which excludes it from Δ.
pub struct DecryptingParty<'a, const W: usize> {
    /// This party's decryption factors in Verificatum's convention,
    /// `u^{−x_l/α}`, one per ciphertext.
    pub factors: &'a [[P256Element; W]],
    /// The batched proof of correctness.
    pub proof: &'a BatchedDecryptionProof<W>,
    /// Whether this party is in Δ. Exactly λ parties must be true, and Δ is
    /// taken as the **first** λ true flags.
    pub participated: bool,
}

/// A `type = mixing` proof: a shuffle chain followed by threshold decryption.
pub struct MixingProof<'a, const W: usize> {
    /// The shuffle half, identical to a shuffling proof's.
    pub shuffle: ShufflingProof<'a, W>,
    /// The decrypted plaintexts, undecoded.
    pub plaintexts: &'a [[P256Element; W]],
    /// Every party, in index order starting at 1.
    pub parties: &'a [DecryptingParty<'a, W>],
}

impl<const W: usize> MixingProof<'_, W> {
    /// Write the proof directory at `dir`.
    pub fn write(&self, dir: &Path) -> Result<()> {
        // The shuffle half first, then correct `type` and add the decryption
        // artifacts on top.
        self.shuffle.write(dir)?;
        let proofs = dir.join("proofs");
        write_ascii(&dir.join("type"), "mixing")?;

        // A mixing proof publishes plaintexts rather than shuffled ciphertexts.
        let _ = fs::remove_file(dir.join("ShuffledCiphertexts.bt"));
        write_tree(
            &dir.join("Plaintexts.bt"),
            &encode::component_array_to_tree(self.plaintexts)?,
        )?;

        let participating = self.parties.iter().filter(|p| p.participated).count();
        if participating != self.shuffle.threshold {
            return Err(anyhow!(
                "{participating} parties marked correct but the threshold is {}",
                self.shuffle.threshold
            ));
        }

        // CorrectIndices is a boolean array of length k+1; entry 0 is ignored.
        let mut flags = Vec::with_capacity(self.parties.len() + 1);
        flags.push(false);
        flags.extend(self.parties.iter().map(|p| p.participated));
        write_tree(
            &proofs.join("CorrectIndices.bt"),
            &crate::wire::arithm::bool_array(&flags),
        )?;

        for (index, party) in self.parties.iter().enumerate() {
            let l = index + 1;
            write_tree(
                &proofs.join(format!("DecryptionFactors{l:02}.bt")),
                &encode::component_array_to_tree(party.factors)?,
            )?;
            write_tree(
                &proofs.join(format!("DecrFactCommitment{l:02}.bt")),
                &ByteTree::node(vec![
                    encode::element_to_tree(&party.proof.y_prime)?,
                    encode::elements_to_tree(&party.proof.b_prime)?,
                ]),
            )?;
            write_tree(
                &proofs.join(format!("DecrFactReply{l:02}.bt")),
                &encode::scalar_to_tree(&party.proof.k_x)?,
            )?;
        }

        Ok(())
    }
}
