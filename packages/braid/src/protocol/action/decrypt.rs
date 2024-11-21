#![allow(clippy::too_many_arguments)]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::protocol::datalog;
use anyhow::Result;
use rayon::prelude::*;
use strand::{serialization::StrandVector, zkp::ChaumPedersen};

/// Computes the decryption factors using this trustee's secret share.
///
/// The plaintexts can be calculated from a threshold number of
/// decryption factors. Each ciphertext produces one decryption
/// factor and one proof of discrete log equality.
///
/// Returns a Message of type DecryptionFactors signed by
/// this trustee.
///
/// As described in Cortier et al.; based on Pedersen.
pub(super) fn compute_decryption_factors<C: Ctx>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    channels_hs: &ChannelsHashes,
    ciphertexts_h: &CiphertextsHash,
    mix_signer: &TrusteePosition,
    pk_h: &PublicKeyHash,
    shares_hs: &SharesHashes,
    self_p: &TrusteePosition,
    num_t: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>, ProtocolError> {
    let ctx = C::default();
    let cfg = trustee.get_configuration(cfg_h)?;

    let pk = trustee
        .get_dkg_public_key(pk_h, 0)
        .add_context("Computing decryption factors")?;
    let vk = pk.verification_keys[*self_p].clone();

    let my_channel = trustee
        .get_channel(&ChannelHash(channels_hs.0[*self_p]), *self_p)
        .add_context("Computing decryption factors")?;

    let mut secret = C::X::add_identity();
    for sender in 0..*num_t {
        let share_h = shares_hs.0[sender];
        let share_ = trustee
            .get_shares(&SharesHash(share_h), sender)
            .add_context("Computing decryption factors")?;

        let sk = trustee.decrypt_share_sk(&my_channel, &cfg)?;

        let share = ctx.decrypt_exp(&share_.encrypted_shares[*self_p], sk)?;

        secret = secret.add(&share);
        secret = secret.modq(&ctx);
    }

    let ciphertexts = trustee
        .get_mix(ciphertexts_h, *batch, *mix_signer)
        .add_context("Computing decryption factors")?;

    info!(
        "ComputeDecryptionFactors [{}] ({})..",
        dbg_hash(&ciphertexts_h.0),
        ciphertexts.ciphertexts.0.len(),
    );

    let suffix = format!("decryption_factor{self_p}");
    let label = cfg.label(*batch, suffix);

    let zkp = strand::zkp::Zkp::new(&ctx);

    let result: Result<Vec<(C::E, ChaumPedersen<C>)>, ProtocolError> = ciphertexts
        .ciphertexts
        .0
        .par_iter()
        .map(|c| {
            let (base, proof) =
                strand::threshold::decryption_factor(&c, &secret, &vk, &label, &zkp, &ctx)?;

            // FIXME removed self-verify
            // let ok = zkp.verify_decryption(&vk, &base, &c.mhr, &c.gr, &proof, &label);
            // assert!(ok);

            Ok((base, proof))
        })
        .collect();

    let (factors, proofs): (Vec<C::E>, Vec<ChaumPedersen<C>>) = result?.into_iter().unzip();

    let df = DecryptionFactors::new(factors, StrandVector(proofs));
    let m = Message::decryption_factors_msg(cfg, *batch, df, *ciphertexts_h, *shares_hs, trustee)?;
    Ok(vec![m])
}

/// Computes the plaintexts from a threshold number of decryption factors.
///
/// Includes verification of decryption proofs. Returns a Message of type
/// Plaintexts signed by this trustee.
pub(super) fn compute_plaintexts<C: Ctx>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    pk_h: &PublicKeyHash,
    dfactors_hs: &DecryptionFactorsHashes,
    ciphertexts_h: &CiphertextsHash,
    mix_signer: &TrusteePosition,
    ts: &TrusteeSet,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>, ProtocolError> {
    let cfg = trustee.get_configuration(cfg_h)?;
    let plaintexts = compute_plaintexts_(
        cfg_h,
        batch,
        pk_h,
        dfactors_hs,
        ciphertexts_h,
        mix_signer,
        ts,
        threshold,
        trustee,
    )?;
    let m = Message::plaintexts_msg(
        cfg,
        *batch,
        plaintexts,
        *dfactors_hs,
        *ciphertexts_h,
        *pk_h,
        trustee,
    )?;

    Ok(vec![m])
}

/// Verifies the plaintexts by re-computing the plaintexts independently.
///
/// Includes verification of decryption proofs. Returns a Message of type
/// PlaintextsSigned signed by this trustee.
pub(super) fn sign_plaintexts<C: Ctx>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    pk_h: &PublicKeyHash,
    plaintexts_h: &PlaintextsHash,
    dfactors_hs: &DecryptionFactorsHashes,
    ciphertexts_h: &CiphertextsHash,
    mix_signer: &TrusteePosition,
    trustees: &TrusteeSet,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>, ProtocolError> {
    let cfg = trustee.get_configuration(cfg_h)?;
    info!(
        "SignPlaintexts verifying decryption [{}] => [{}]",
        dbg_hash(&ciphertexts_h.0),
        dbg_hash(&plaintexts_h.0),
    );

    let expected = compute_plaintexts_(
        cfg_h,
        batch,
        pk_h,
        dfactors_hs,
        ciphertexts_h,
        mix_signer,
        trustees,
        threshold,
        trustee,
    )?;
    let actual = trustee
        .get_plaintexts(plaintexts_h, *batch, trustees[0] - 1)
        .add_context("Signing plaintexts")?;

    if expected.0 .0 == actual.0 .0 {
        info!(
            "SignPlaintexts verifying decryption [{}] => [{}], ok",
            dbg_hash(&ciphertexts_h.0),
            dbg_hash(&plaintexts_h.0),
        );
        let m = Message::plaintexts_signed_msg(
            cfg,
            *batch,
            *plaintexts_h,
            *dfactors_hs,
            *ciphertexts_h,
            *pk_h,
            trustee,
        )
        .add_context("Signing plaintexts")?;

        Ok(vec![m])
    } else {
        Err(ProtocolError::VerificationError(format!(
            "Mismatch when comparing plaintexts with retrieved ones"
        )))
    }
}

/// Computes the plaintexts from a threshold number of decryption factors.
///
/// For each ciphertext and trustee, verifies the decryption factors, then
/// combines them into a single divisor. This divisor is then applied to
/// the mhr part of the ciphertext to yield the plaintext.
///
/// Returns a Message of type Plaintexts signed by this trustee.
///
/// As described in Cortier et al.; based on Pedersen.
fn compute_plaintexts_<C: Ctx>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    pk_h: &PublicKeyHash,
    dfactors_hs: &DecryptionFactorsHashes,
    ciphertexts_h: &CiphertextsHash,
    mix_signer: &TrusteePosition,
    ts: &TrusteeSet,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Plaintexts<C>, ProtocolError> {
    let ctx = C::default();
    let cfg = trustee.get_configuration(cfg_h)?;
    let zkp = strand::zkp::Zkp::new(&ctx);
    let pk = trustee
        .get_dkg_public_key(pk_h, 0)
        .add_context("Computing plaintexts")?;

    let mix = trustee
        .get_mix(ciphertexts_h, *batch, *mix_signer)
        .add_context("Computing plaintexts")?;

    let num_ciphertexts = mix.ciphertexts.0.len();
    let mut divider = vec![C::E::mul_identity(); num_ciphertexts];

    info!(
        "ComputePlaintexts [{}] ({})..",
        dbg_hash(&ciphertexts_h.0),
        num_ciphertexts,
    );

    assert_eq!(
        datalog::hashes_count(&dfactors_hs.0),
        *threshold,
        "Unexpected number of decryption factors"
    );

    // Decryption factors for each trustee
    for (t, df_h) in dfactors_hs.0.iter().enumerate() {
        // Threshold is 1-based
        if t < *threshold {
            let dfactors = trustee
                .get_decryption_factors(&DecryptionFactorsHash(*df_h), *batch, ts[t] - 1)
                .add_context("Computing plaintexts")?;

            assert_eq!(num_ciphertexts, dfactors.factors.0.len());
            let vk = pk.verification_keys[ts[t] - 1].clone();

            // Lagrange parameter is 1-based, as is the ts[] array. The set of present trustees is generated by datalog
            // as a fixed sized array with padded zeroes, so we select the slice corresponding to the
            // filled in trustees.
            let lagrange = strand::threshold::lagrange(ts[t], &ts[0..*threshold], &ctx);

            let it = dfactors
                .factors
                .0
                .par_iter()
                .zip(dfactors.proofs.0.par_iter());
            let it2 = it.zip(mix.ciphertexts.0.par_iter());

            let suffix = format!("decryption_factor{}", ts[t] - 1);
            let label = cfg.label(*batch, suffix);

            let values: Result<Vec<C::E>, ProtocolError> = it2
                .into_par_iter()
                .map(|((df, proof), c)| {
                    let ok = strand::threshold::verify_decryption_factor(
                        &c, &vk, &df, &proof, &label, &zkp,
                    )?;
                    if ok {
                        Ok(ctx.emod_pow(&df, &lagrange))
                    } else {
                        Err(ProtocolError::VerificationError(format!(
                            "Failed to verify decryption proof"
                        )))
                    }
                })
                .collect();

            for (index, next) in values?.iter().enumerate() {
                divider[index] = divider[index].mul(next).modp(&ctx);
            }
        } else {
            debug!("Processed all decryption factors (t = {})", t);
            break;
        }
    }

    info!(
        "ComputePlaintexts applying decryption factors[{}] ({})..",
        dbg_hash(&ciphertexts_h.0),
        num_ciphertexts,
    );
    let ps = mix
        .ciphertexts
        .0
        .par_iter()
        .enumerate()
        .map(|(index, c)| {
            let decrypted = c.mhr.divp(&divider[index], &ctx).modp(&ctx);

            ctx.decode(&decrypted)
        })
        .collect();

    Ok(Plaintexts(StrandVector(ps)))
}
