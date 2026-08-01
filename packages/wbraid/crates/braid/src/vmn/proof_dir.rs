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
//! - `PolynomialInExponent.bt` is **not** read for a shuffling proof, despite
//!   VMNV §9.3 step 5 describing key reading as unconditional. Verified against
//!   the implementation: deleting the file leaves `vmnv -shuffle` at exit 0, and
//!   "Read polynomial in exponent" appears only in a `-mix` run. It is written
//!   anyway when a caller supplies it, since a conforming verifier may expect a
//!   complete directory, but a shuffling proof does not need a DKG to exist.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};

use cryptography::context::P256Ctx;
use cryptography::cryptosystem::elgamal::Ciphertext;
use cryptography::groups::p256::element::P256Element;
use cryptography::zkp::shuffle::ShuffleProof;

use vcompat::bytetree::ByteTree;

use super::{challenges::commitments_to_tree, encode};

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
    /// Input ciphertexts `L_0`.
    pub input: &'a [Ciphertext<P256Ctx, W>],
    /// The mixers in order, each consuming the previous list. Must be non-empty;
    /// its length is the active threshold `λ_a`, and the last output is the
    /// session result `L_λa`.
    pub mixers: &'a [MixerStep<'a, W>],
    /// The polynomial in the exponent, if known. Unused by `vmnv -shuffle` (see
    /// the module docs), so a shuffling session need not run a DKG.
    pub polynomial_in_exponent: Option<&'a [P256Element]>,
}

impl<const W: usize> ShufflingProof<'_, W> {
    /// Write the proof directory at `dir`, creating it if absent.
    pub fn write(&self, dir: &Path) -> Result<()> {
        if self.mixers.is_empty() {
            return Err(anyhow!("a shuffling proof needs at least one mixer"));
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
