#![allow(clippy::too_many_arguments)]
use super::*;
use anyhow::anyhow;
use anyhow::Result;
use strand::elgamal::{PrivateKey, PublicKey};

pub(super) fn gen_commitments<C: Ctx>(
    configuration_hash: &ConfigurationHash,
    threshold: TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>> {
    let ctx: C = Default::default();

    let cfg = trustee.get_configuration(configuration_hash)?;
    let (coefficients, commitments) = strand::threshold::gen_coefficients(threshold, &ctx);
    let ec = trustee.encrypt_coefficients(coefficients)?;

    // Generate a public key for share transport
    let sk = ctx.rnd_exp();
    let pk = ctx.gmod_pow(&sk);
    let st = trustee.encrypt_share_sk(pk, sk)?;

    let commitments: Commitments<C> = Commitments::new(commitments, ec, st);
    let m = Message::commitments_msg(cfg, &commitments, true, trustee)?;
    Ok(vec![m])
}

// FIXME Sign the commitments only if they contain our commitments in the right position
pub(super) fn sign_commitments<C: Ctx>(
    configuration_h: &ConfigurationHash,
    commitments_hs: &CommitmentsHashes,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>> {
    let cfg = trustee.get_configuration(configuration_h)?;

    for (i, h) in commitments_hs
        .0
        .iter()
        .filter(|h| **h != NULL_HASH)
        .enumerate()
    {
        let hash = *h;
        let commitments = trustee.get_commitments(&CommitmentsHash(hash), i);
        // FIXME assert
        assert!(commitments.is_some());
    }
    // The commitments hashes will be grouped into one sequence of bytes when
    // constructing the parameter target.
    let m = Message::commitments_all_signed_msg(cfg, commitments_hs, trustee)?;
    Ok(vec![m])
}

pub(super) fn compute_shares<C: Ctx>(
    configuration_h: &ConfigurationHash,
    commitments_hs: &CommitmentsHashes,
    self_p: &TrusteePosition,
    num_trustees: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>> {
    let ctx = C::default();
    let cfg = trustee.get_configuration(configuration_h)?;

    // Get our own commitments
    let my_commitments_h = commitments_hs
        .0
        .get(*self_p)
        .ok_or(anyhow!("Could not retrieve commitments hash for self",))?;

    let hash = *my_commitments_h;
    let commitments_ = trustee.get_commitments(&CommitmentsHash(hash), *self_p);
    let commitments = commitments_.ok_or(anyhow!("Could not find my commitments",))?;

    trace!("Found commitments for self {:?}", commitments);
    let coeffs = trustee
        .decrypt_coefficients(&commitments.encrypted_coefficients)
        .ok_or(anyhow!("Could not decrypt coefficients",))?;

    let mut s = vec![];

    for i in 0..*num_trustees {
        let share = strand::threshold::eval_poly(i + 1, *threshold, &coeffs, &ctx);

        // Obtain the public key for the recipient of the share
        let target_commitments_h = commitments_hs
            .0
            .get(i)
            .ok_or(anyhow!("Could not retrieve commitments hash",))?;

        let target_hash = *target_commitments_h;

        let target_commitments = trustee
            .get_commitments(&CommitmentsHash(target_hash), i)
            .ok_or(anyhow!("Could not retrieve commitments"))?;

        // Encrypt share for target trustee
        let encryption_pk =
            PublicKey::<C>::from_element(&target_commitments.share_transport.pk, &ctx);

        let share_bytes = ctx.encrypt_exp(&share, encryption_pk);

        s.push(share_bytes?)
    }

    let shares = Shares(s);
    let m = Message::shares_msg(cfg, &shares, trustee)?;
    Ok(vec![m])
}

pub(super) fn compute_pk<C: Ctx>(
    cfg_h: &ConfigurationHash,
    shares_hs: &SharesHashes,
    commitments_hs: &CommitmentsHashes,
    self_pos: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>> {
    let cfg = trustee.get_configuration(cfg_h)?;
    let pk_ = compute_pk_(
        cfg_h,
        shares_hs,
        commitments_hs,
        self_pos,
        num_t,
        threshold,
        trustee,
    );
    if let Ok(pk) = pk_ {
        let public_key: DkgPublicKey<C> = DkgPublicKey::new(pk.0, pk.1);

        // The shares and commitments hashes will be grouped into one sequence of bytes when
        // constructing the parameter target.
        let m =
            Message::public_key_msg(cfg, &public_key, shares_hs, commitments_hs, true, trustee)?;
        Ok(vec![m])
    } else {
        Err(anyhow!("Could not compute pk"))
    }
}

pub(super) fn sign_pk<C: Ctx>(
    cfg_h: &ConfigurationHash,
    pk_h: &PublicKeyHash,
    shares_hs: &SharesHashes,
    commitments_hs: &CommitmentsHashes,
    self_pos: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>> {
    let cfg = trustee.get_configuration(cfg_h)?;
    info!(
        "SignPk verifying public key [{}] ({})..",
        dbg_hash(&pk_h.0),
        num_t,
    );

    let expected = compute_pk_(
        cfg_h,
        shares_hs,
        commitments_hs,
        self_pos,
        num_t,
        threshold,
        trustee,
    )?;

    let actual = trustee
        .get_dkg_public_key(pk_h, 0)
        .ok_or(anyhow!("Could not retrieve dkg public key",))?;

    if (expected.0 == actual.pk) && (expected.1 == actual.verification_keys) {
        info!(
            "SignPk verifying public key [{}] ({}), ok",
            dbg_hash(&pk_h.0),
            num_t,
        );
        let m = Message::public_key_msg(cfg, &actual, shares_hs, commitments_hs, false, trustee)?;
        Ok(vec![m])
    } else {
        Err(anyhow!(
            "Mismatch when comparing computed public key with retrieved one",
        ))
    }
}

fn compute_pk_<C: Ctx>(
    _cfg_h: &ConfigurationHash,
    shares_hs: &SharesHashes,
    commitments_hs: &CommitmentsHashes,
    self_p: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C>,
) -> Result<(C::E, Vec<C::E>)> {
    let ctx = C::default();
    let mut pk = C::E::mul_identity();
    let mut verification_keys = vec![C::E::mul_identity(); *num_t];

    // Iterate over sender trustee commitments
    for (i, h) in commitments_hs
        .0
        .iter()
        .filter(|h| **h != NULL_HASH)
        .enumerate()
    {
        let hash = *h;
        let commitments_ = trustee.get_commitments(&CommitmentsHash(hash), i);
        let commitments = commitments_.ok_or(anyhow!("Failed to retrieve commitments",))?;

        pk = pk.mul(&commitments.commitments[0]).modp(&ctx);

        // Get share corresponding to the sender of the commitments
        let share_h = shares_hs.0[i];
        let share_ = trustee.get_shares(&SharesHash(share_h), i);

        let share = share_.ok_or(anyhow!("Failed to retrieve shares",))?;

        // Iterate over receiver trustees to compute their verification key
        for (j, vk) in verification_keys.iter_mut().enumerate().take(*num_t) {
            let vkf = strand::threshold::verification_key_factor(
                &commitments.commitments,
                *threshold,
                j,
                &ctx,
            );
            // verification_keys[j] = verification_keys[j].mul(&vkf).modulo(ctx.modulus());
            *vk = vk.mul(&vkf).modp(&ctx);

            // Our share is sent from trustee i to j, when j = us
            if j == *self_p {
                // Construct our private key to decrypt our share
                let my_commitments_h = commitments_hs
                    .0
                    .get(*self_p)
                    .ok_or(anyhow!("Could not retrieve commitments hashes for self"))?;

                let my_commitments = trustee
                    .get_commitments(&CommitmentsHash(*my_commitments_h), *self_p)
                    .ok_or(anyhow!("Could not retrieve commitments for self",))?;

                let sk = trustee
                    .decrypt_share_sk(&my_commitments.share_transport)
                    .ok_or(anyhow!("Could not decrypt share transport",))?;
                let sk = PrivateKey::from(&sk, &ctx);

                // Decrypt the share sent from i to us
                let value = ctx.decrypt_exp(&share.0[*self_p], sk)?;

                // FIXME assert
                assert_eq!(ctx.gmod_pow(&value), vkf);
                info!("Trustee {} verified share received from {}", j, i);
            }
        }
    }

    Ok((pk, verification_keys))
}
