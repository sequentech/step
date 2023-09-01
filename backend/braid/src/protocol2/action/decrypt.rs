#![allow(clippy::too_many_arguments)]
use super::*;
use anyhow::anyhow;
use anyhow::Result;
use rayon::prelude::*;
use strand::{
    elgamal::PrivateKey, serialization::StrandVectorCP, serialization::StrandVectorP,
    zkp::ChaumPedersen,
};

pub(super) fn compute_decryption_factors<C: Ctx>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    commitments_hs: &CommitmentsHashes,
    ciphertexts_h: &CiphertextsHash,
    mix_signer: &TrusteePosition,
    pk_h: &PublicKeyHash,
    shares_hs: &SharesHashes,
    self_p: &TrusteePosition,
    num_t: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>> {
    let ctx = C::default();
    let cfg = trustee.get_configuration(cfg_h)?;

    let pk = trustee
        .get_dkg_public_key(pk_h, 0)
        .ok_or(anyhow!("Could not retrieve dkg public key",))?;
    let vk = pk.verification_keys[*self_p].clone();

    let ciphertexts = trustee
        .get_mix(ciphertexts_h, *batch, *mix_signer)
        .ok_or(anyhow!("Could not retrieve mix"))?;

    let commitments = trustee
        .get_commitments(&CommitmentsHash(commitments_hs.0[*self_p]), *self_p)
        .ok_or(anyhow!("Could not retrieve commitments",))?;

    let mut secret = C::X::add_identity();
    for sender in 0..*num_t {
        let share_h = shares_hs.0[sender];
        let share_ = trustee
            .get_shares(&SharesHash(share_h), sender)
            .ok_or(anyhow!("Could not retrieve shares",))?;

        let sk = trustee
            .decrypt_share_sk(&commitments.share_transport)
            .ok_or(anyhow!("Could not decrypt share transport",))?;
        let sk = PrivateKey::from(&sk, &ctx);

        let share = ctx.decrypt_exp(&share_.0[*self_p], sk)?;

        secret = secret.add(&share);
        secret = secret.modq(&ctx);
    }

    info!(
        "Computing {} decryption factors..",
        ciphertexts.ciphertexts.0.len()
    );

    let suffix = format!("decryption_factor{self_p}");
    let label = cfg.label(*batch, suffix);

    let (factors, proofs): (Vec<C::E>, Vec<ChaumPedersen<C>>) = ciphertexts
        .ciphertexts
        .0
        .into_par_iter()
        .map(|c| {
            // FIXME unwrap
            let (base, proof) =
                strand::threshold::decryption_factor(&c, &secret, &vk, &label, ctx.clone())
                    .unwrap();

            // FIXME removed self-verify
            // let ok = zkp.verify_decryption(&vk, &base, &c.mhr, &c.gr, &proof, &label);
            // assert!(ok);

            (base, proof)
        })
        .unzip();

    let df = DecryptionFactors::new(factors, StrandVectorCP(proofs));
    let m = Message::decryption_factors_msg(cfg, *batch, df, *ciphertexts_h, *shares_hs, trustee)?;
    Ok(vec![m])
}

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
) -> Result<Vec<Message>> {
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
) -> Result<Vec<Message>> {
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
        .ok_or(anyhow!("Could not retrieve plaintexts".to_string(),))?;

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
        )?;
        Ok(vec![m])
    } else {
        Err(anyhow!(
            "Mismatch when comparing plaintexts with retrieved ones",
        ))
    }
}

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
) -> Result<Plaintexts<C>> {
    let ctx = C::default();
    let cfg = trustee.get_configuration(cfg_h)?;
    let zkp = strand::zkp::Zkp::new(&ctx);
    let pk = trustee
        .get_dkg_public_key(pk_h, 0)
        .ok_or(anyhow!("Could not retrieve dkg public key".to_string(),))?;

    let mix = trustee
        .get_mix(ciphertexts_h, *batch, *mix_signer)
        .ok_or(anyhow!("Could not retrieve mix".to_string()))?;
    let num_ciphertexts = mix.ciphertexts.0.len();
    let mut divider = vec![C::E::mul_identity(); num_ciphertexts];

    info!(
        "ComputePlaintexts [{}] ({})..",
        dbg_hash(&ciphertexts_h.0),
        num_ciphertexts,
    );

    // Decryption factors for each trustee
    for (t, df_h) in dfactors_hs.0.iter().enumerate() {
        // Threshold is 1-based
        if t < *threshold {
            let dfactors = trustee
                .get_decryption_factors(&DecryptionFactorsHash(*df_h), *batch, ts[t] - 1)
                .ok_or(anyhow!("Could not retrieve decryption factors".to_string(),))?;

            assert_eq!(num_ciphertexts, dfactors.factors.0.len());
            let vk = pk.verification_keys[ts[t] - 1].clone();

            // Lagrange parameter is 1-based, as is the ts[] array. The set of present trustees is generated by datalog
            // as a fixed sized array with padded zeroes, so we select the slice corresponding to the
            // filled in trustees.
            let lagrange = strand::threshold::lagrange(ts[t], &ts[0..*threshold], &ctx);

            let ciphertexts = mix.ciphertexts.clone();

            let it = dfactors
                .factors
                .0
                .into_par_iter()
                .zip(dfactors.proofs.0.into_par_iter());
            let it2 = it.zip(ciphertexts.0.into_par_iter());

            let suffix = format!("decryption_factor{}", ts[t] - 1);
            let label = cfg.label(*batch, suffix);

            let values: Vec<C::E> = it2
                .into_par_iter()
                .map(|((df, proof), c)| {
                    // FIXME unwrap
                    let ok = zkp
                        .verify_decryption(&vk, &df, &c.mhr, &c.gr, &proof, &label)
                        .unwrap();
                    // FIXME assert
                    assert!(ok);
                    ctx.emod_pow(&df, &lagrange)
                })
                .collect();

            for (index, next) in values.iter().enumerate() {
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
        .into_par_iter()
        .enumerate()
        .map(|(index, c)| {
            let decrypted = c.mhr.divp(&divider[index], &ctx).modp(&ctx);

            ctx.decode(&decrypted)
        })
        .collect();

    Ok(Plaintexts(StrandVectorP(ps)))
}
