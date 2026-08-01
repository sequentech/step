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
//! - The verifier reads the **keys** unconditionally (VMNV §9.3 step 5), even
//!   for a shuffle-only proof that never decrypts, so `PolynomialInExponent.bt`
//!   must be present. For a single party with threshold 1 it is the one-element
//!   array `(Gamma_0)` with `Gamma_0 = y`.
//! - With one mix server the final output `ShuffledCiphertexts.bt` and that
//!   server's intermediate output `Ciphertexts01.bt` are the same list, but both
//!   files are required.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};

use cryptography::context::P256Ctx;
use cryptography::cryptosystem::elgamal::Ciphertext;
use cryptography::groups::p256::element::P256Element;
use cryptography::zkp::shuffle::ShuffleProof;

use vcompat::bytetree::ByteTree;

use super::{challenges::commitments_to_tree, encode};

/// Everything needed to write a one-server shuffling proof.
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
    /// Output ciphertexts `L_1`, which for one server is also `L_lambda_a`.
    pub output: &'a [Ciphertext<P256Ctx, W>],
    /// The proof of a shuffle relating them.
    pub proof: &'a ShuffleProof<P256Ctx, W>,
}

impl<const W: usize> ShufflingProof<'_, W> {
    /// Write the proof directory at `dir`, creating it if absent.
    pub fn write(&self, dir: &Path) -> Result<()> {
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
        let output_tree = encode::ciphertexts_to_tree(self.output)?;
        write_tree(&dir.join("ShuffledCiphertexts.bt"), &output_tree)?;

        // --- proofs: keys and intermediate values ------------------------
        write_ascii(&proofs.join("activethreshold"), "1")?;
        // Threshold 1: the Shamir polynomial in the exponent is just (y).
        write_tree(
            &proofs.join("PolynomialInExponent.bt"),
            &ByteTree::node(vec![encode::element_to_tree(self.public_key)?]),
        )?;
        write_tree(&proofs.join("Ciphertexts01.bt"), &output_tree)?;

        // --- proofs: the proof of a shuffle ------------------------------
        let commitments = &self.proof.commitments;
        write_tree(
            &proofs.join("PermutationCommitment01.bt"),
            &encode::elements_to_tree(commitments.u_n())?,
        )?;
        write_tree(
            &proofs.join("PoSCommitment01.bt"),
            &commitments_to_tree(commitments)
                .map_err(|e| anyhow!("failed to encode proof commitment: {e:?}"))?,
        )?;
        write_tree(&proofs.join("PoSReply01.bt"), &responses_to_tree(self.proof)?)?;

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
