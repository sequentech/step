// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Reading a proof directory and verifying the session it describes.
//!
//! [`crate::proof_dir`] is the writing half — turning our own execution into the
//! layout Verificatum expects. This is the reading half: taking a directory
//! Verificatum (or we) produced and checking every proof in it, which is VMNV
//! Algorithm 28.
//!
//! # Supported subset, declared rather than assumed
//!
//! Algorithm 28 covers more than this does. What is *not* implemented is
//! rejected explicitly, never skipped:
//!
//! - `type = mixing` and `type = shuffling` only; `decryption` sessions are
//!   refused because we have never seen one and would be guessing.
//! - Non-interactive Fiat–Shamir proofs only, `posc` and `ccpos` both implied by
//!   the presence of the ordinary proof-of-shuffle files. Commitment-consistent
//!   shuffles (Algorithm 21) and pre-computation modes are refused.
//! - P-256 only, since that is the group the encoding layer implements.
//!
//! Refusing is the point. `vmnv` has three separate places where a condition is
//! evaluated and the conclusion is not enforced — see `VERIFICATUM.md` — and the
//! failure mode that produces is a verifier reporting success over checks it did
//! not perform.
//!
//! # Mixer slots are indexed by party
//!
//! Not sequentially. A party that took no part in shuffling leaves a gap: with
//! the active set `{1,3}` the directory holds `PermutationCommitment01` and
//! `03` and no `02`, while `activethreshold` is `3` — the highest active index,
//! not the count. `vmnv` decides by file existence and skips a missing slot
//! silently. We skip it too, because otherwise valid proofs are rejected, but
//! [`Outcome::mixers_verified`] reports how many were actually checked so a
//! caller can insist that a chain is as long as the session claims.

use std::path::Path;

use anyhow::{anyhow, Context as _, Result};

use cryptography::context::P256Ctx;
use cryptography::cryptosystem::elgamal::{Ciphertext, PublicKey};
use cryptography::groups::p256::element::P256Element;
use cryptography::traits::groups::GroupElement;
use cryptography::zkp::shuffle::{Responses, ShuffleCommitments, ShuffleProof, Shuffler};

use crate::challenges::VmnChallenges;
use crate::decrypt::BatchedDecryptionProof;
use crate::generators::vmn_generators;
use crate::verify::{verify_decryption, PartyContribution, SessionParams};
use crate::wire::arithm::bool_array_values;
use crate::wire::bytetree::ByteTree;
use crate::wire::crypto::{global_prefix, Hashfunction};
use crate::wire::protinfo::ProtocolInfo;
use crate::encode;

/// The kind of session a proof directory describes (VMNV §9.1, `type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofType {
    /// A shuffle chain followed by threshold decryption.
    Mixing,
    /// A shuffle chain with no decryption.
    Shuffling,
}

/// What a proof directory declares about itself, before any cryptography.
#[derive(Clone, Debug)]
pub struct ProofMetadata {
    pub version: String,
    pub proof_type: ProofType,
    pub auxsid: String,
    /// Ciphertext width `ω`. May differ from the protocol info file's default.
    pub width: usize,
    /// `λ_a`. Because slots are party-indexed, this is the highest active party
    /// index rather than the number of mixers.
    pub active_threshold: usize,
}

/// What verification established.
#[derive(Clone, Debug)]
pub struct Outcome<const W: usize> {
    /// How many mixers actually had a proof and passed it. Compare against the
    /// session threshold: a shorter chain is accepted by `vmnv` in silence.
    pub mixers_verified: usize,
    /// The session output — the last mixer's list.
    pub shuffled: Vec<Ciphertext<P256Ctx, W>>,
    /// The plaintexts implied by the decryption proof, for a mixing session.
    pub plaintexts: Option<Vec<[P256Element; W]>>,
}

/// Read what the directory says about itself.
///
/// # Errors
///
/// Refuses a session type or version this implementation does not cover.
pub fn read_metadata(dir: &Path) -> Result<ProofMetadata> {
    let text = |name: &str| -> Result<String> {
        std::fs::read_to_string(dir.join(name))
            .with_context(|| format!("reading {name}"))
            .map(|s| s.trim().to_string())
    };

    let version = text("version")?;
    if version != "3.1.0" {
        return Err(anyhow!(
            "proof claims VMN version {version}; only 3.1.0 is implemented"
        ));
    }

    let declared = text("type")?;
    let proof_type = match declared.as_str() {
        "mixing" => ProofType::Mixing,
        "shuffling" => ProofType::Shuffling,
        // Refused rather than treated as one of the above: we have never seen a
        // decryption session and would be guessing at its layout.
        other => return Err(anyhow!("unsupported session type {other:?}")),
    };

    Ok(ProofMetadata {
        version,
        proof_type,
        auxsid: text("auxsid")?,
        width: text("width")?
            .parse()
            .map_err(|_| anyhow!("width is not a number"))?,
        active_threshold: text("proofs/activethreshold")?
            .parse()
            .map_err(|_| anyhow!("activethreshold is not a number"))?,
    })
}

/// Verify every proof in a session.
///
/// `W` must equal `meta.width`; the caller dispatches, since the width is only
/// known at runtime but the ciphertext type is const-generic.
///
/// # Errors
///
/// Returns `Err` when the directory cannot be read or is malformed, and
/// `Ok(None)` when it is well formed but a proof does not verify — so a caller
/// cannot mistake "could not check" for "checked and passed".
pub fn verify_session<const W: usize>(
    dir: &Path,
    info: &ProtocolInfo,
    meta: &ProofMetadata,
) -> Result<Option<Outcome<W>>> {
    if W != meta.width {
        return Err(anyhow!(
            "verifying at width {W} a proof that declares width {}",
            meta.width
        ));
    }

    let rho = global_prefix(Hashfunction::Sha256, &info.prefix_params(&meta.auxsid));
    let hash = Hashfunction::Sha256;

    let pk_tree = tree(dir, "FullPublicKey.bt")?;
    let y = encode::tree_to_element(
        &pk_tree
            .as_node_of(2)
            .map_err(|e| anyhow!("FullPublicKey is not (g, y): {e}"))?[1],
    )?;

    // --- the shuffle chain -------------------------------------------------
    let mut current = encode::tree_to_ciphertexts::<W>(&tree(dir, "Ciphertexts.bt")?)?;
    let generators = vmn_generators(hash, &rho, info.n_r as usize, current.len())?;
    let mut mixers_verified = 0;

    for slot in 1..=meta.active_threshold {
        if !dir
            .join(format!("proofs/PermutationCommitment{slot:02}.bt"))
            .is_file()
        {
            // The party at this index took no part. See the module docs.
            continue;
        }

        let output =
            encode::tree_to_ciphertexts::<W>(&tree(dir, &format!("proofs/Ciphertexts{slot:02}.bt"))?)?;
        let proof = read_shuffle_proof::<W>(dir, slot)?;

        let shuffler = Shuffler::<P256Ctx, W>::new(generators.clone(), PublicKey::<P256Ctx>::new(y));
        let challenges = VmnChallenges::new(
            hash,
            rho.clone(),
            info.n_e as usize,
            info.n_v as usize,
            W,
        );
        let ok = shuffler
            .verify_with(&current, &output, &proof, &[], &challenges)
            .map_err(|e| anyhow!("shuffle verification failed for mixer {slot}: {e:?}"))?;
        if !ok {
            return Ok(None);
        }

        current = output;
        mixers_verified += 1;
    }

    if meta.proof_type == ProofType::Shuffling {
        return Ok(Some(Outcome {
            mixers_verified,
            shuffled: current,
            plaintexts: None,
        }));
    }

    // --- decryption ---------------------------------------------------------
    let gamma = encode::tree_to_elements(&tree(dir, "proofs/PolynomialInExponent.bt")?)?;
    if !gamma[0].equals(&y) {
        // Algorithm 24's cross-check. `vmnv -shuffle` skips it; we do not.
        return Ok(None);
    }

    let correct = bool_array_values(&tree(dir, "proofs/CorrectIndices.bt")?)
        .map_err(|e| anyhow!("CorrectIndices is malformed: {e}"))?;

    let held: Vec<_> = (1..=info.parties)
        .map(|party| read_contribution::<W>(dir, party))
        .collect::<Result<Vec<_>>>()?;
    let contributions: Vec<PartyContribution<W>> = held
        .iter()
        .map(|(factors, proof)| PartyContribution { factors, proof })
        .collect();

    let params = SessionParams {
        rho,
        hash,
        n_e: info.n_e as usize,
        n_v: info.n_v as usize,
        parties: info.parties,
        threshold: info.threshold,
    };

    let Some(plaintexts) = verify_decryption(&params, &gamma, &current, &contributions, &correct)?
    else {
        return Ok(None);
    };

    // Algorithm 28 keeps this separate from the proof, and so do we: the proof
    // establishes the factors, and this establishes that the published output
    // follows from them.
    let published = encode::tree_to_component_array::<W>(&tree(dir, "Plaintexts.bt")?)?;
    if plaintexts != published {
        return Ok(None);
    }

    Ok(Some(Outcome {
        mixers_verified,
        shuffled: current,
        plaintexts: Some(plaintexts),
    }))
}

fn tree(dir: &Path, name: &str) -> Result<ByteTree> {
    let bytes = std::fs::read(dir.join(name)).with_context(|| format!("reading {name}"))?;
    ByteTree::from_bytes(&bytes).map_err(|e| anyhow!("{name} is not a byte tree: {e}"))
}

fn read_contribution<const W: usize>(
    dir: &Path,
    party: usize,
) -> Result<(Vec<[P256Element; W]>, BatchedDecryptionProof<W>)> {
    let factors = encode::tree_to_component_array::<W>(&tree(
        dir,
        &format!("proofs/DecryptionFactors{party:02}.bt"),
    )?)?;

    let tau_tree = tree(dir, &format!("proofs/DecrFactCommitment{party:02}.bt"))?;
    let tau = tau_tree
        .as_node_of(2)
        .map_err(|e| anyhow!("tau^dec is not node(y', B'): {e}"))?;
    let b_prime: [P256Element; W] = encode::tree_to_elements(&tau[1])?
        .try_into()
        .map_err(|_| anyhow!("B' does not have {W} components"))?;

    Ok((
        factors,
        BatchedDecryptionProof {
            y_prime: encode::tree_to_element(&tau[0])?,
            b_prime,
            k_x: encode::tree_to_scalar(&tree(
                dir,
                &format!("proofs/DecrFactReply{party:02}.bt"),
            )?)?,
        },
    ))
}

fn read_shuffle_proof<const W: usize>(
    dir: &Path,
    slot: usize,
) -> Result<ShuffleProof<P256Ctx, W>> {
    let u_n = encode::tree_to_elements(&tree(
        dir,
        &format!("proofs/PermutationCommitment{slot:02}.bt"),
    )?)?;

    let tau_tree = tree(dir, &format!("proofs/PoSCommitment{slot:02}.bt"))?;
    let tau = tau_tree
        .as_node_of(6)
        .map_err(|e| anyhow!("tau^pos does not have 6 components: {e}"))?;

    // F' is a single ciphertext rather than an array, so wrap each component in
    // the transposed one-element form and reuse the array decoder.
    let pair = tau[5]
        .as_node_of(2)
        .map_err(|e| anyhow!("F' is not (u, v): {e}"))?;
    let wrap = |side: &ByteTree| -> Result<ByteTree> {
        Ok(ByteTree::node(
            side.as_node()
                .map_err(|e| anyhow!("F' component: {e}"))?
                .iter()
                .map(|c| ByteTree::node(vec![c.clone()]))
                .collect::<Vec<_>>(),
        ))
    };
    let f_prime = encode::tree_to_ciphertexts::<W>(&ByteTree::node(vec![
        wrap(&pair[0])?,
        wrap(&pair[1])?,
    ]))?
    .remove(0);

    let commitments = ShuffleCommitments::<P256Ctx, W>::new(
        encode::tree_to_elements(&tau[0])?,
        encode::tree_to_element(&tau[1])?,
        encode::tree_to_elements(&tau[2])?,
        encode::tree_to_element(&tau[3])?,
        encode::tree_to_element(&tau[4])?,
        f_prime,
        u_n,
    );

    let sigma_tree = tree(dir, &format!("proofs/PoSReply{slot:02}.bt"))?;
    let sigma = sigma_tree
        .as_node_of(6)
        .map_err(|e| anyhow!("sigma^pos does not have 6 components: {e}"))?;
    let k_f: [_; W] = encode::tree_to_scalars(&sigma[5])?
        .try_into()
        .map_err(|_| anyhow!("k_F does not have {W} entries"))?;

    Ok(ShuffleProof::new(
        commitments,
        Responses::<P256Ctx, W>::new(
            encode::tree_to_scalar(&sigma[0])?,
            encode::tree_to_scalars(&sigma[1])?,
            encode::tree_to_scalar(&sigma[2])?,
            encode::tree_to_scalar(&sigma[3])?,
            encode::tree_to_scalars(&sigma[4])?,
            k_f,
        ),
    ))
}
