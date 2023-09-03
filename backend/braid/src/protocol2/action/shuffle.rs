#![allow(clippy::too_many_arguments)]
use anyhow::anyhow;
use anyhow::Result;

use super::*;

pub(crate) fn mix<C: Ctx>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    source_h: &CiphertextsHash,
    pk_h: &PublicKeyHash,
    signer_t: TrusteePosition,
    mix_no: &MixNumber,
    trustees: &TrusteeSet,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>> {
    let cfg = trustee.get_configuration(cfg_h)?;
    let ctx = C::default();

    let ciphertexts = if *mix_no == 1 {
        assert_eq!(signer_t, PROTOCOL_MANAGER_INDEX);
        // Ballot ciphertexts
        let cs = trustee.get_ballots(source_h, *batch, PROTOCOL_MANAGER_INDEX);
        if let Some(ballots) = cs {
            info!(
                "Mix computing shuffle [{} (ballots)] ({})..",
                dbg_hash(&source_h.0),
                ballots.ciphertexts.0.len()
            );

            Some(ballots.ciphertexts)
        } else {
            error!("Could not retrieve ciphertexts for mixing");
            None
        }
    } else {
        // First mix ciphertexts come from ballots, second from first mix, third from second, etc.
        // mix_no is 1-based, but trustees[] is 0-based, so the previous mixer is
        // the trustee at index n - 2 (= (n - 1) - 1). For example, if we're on mix #2,
        // the source mix is signed by the first trustee, which is trustees[0].
        // Trustees[] elements are 1-based, so n - 1.
        assert_eq!(signer_t, trustees[mix_no - 2] - 1);
        let signer_t = trustees[mix_no - 2] - 1;
        let mix = trustee.get_mix(source_h, *batch, signer_t);
        if let Some(cs) = mix {
            info!(
                "Mix computing shuffle [{} (mix)] ({})..",
                dbg_hash(&source_h.0),
                cs.ciphertexts.0.len()
            );

            Some(cs.ciphertexts)
        } else {
            error!("Could not retrieve ciphertexts for mixing");
            None
        }
    };

    let cs = ciphertexts.ok_or(anyhow!("Could not retrieve public key for mixing",))?;

    let dkg_pk = trustee
        .get_dkg_public_key(pk_h, 0)
        .ok_or(anyhow!("Could not retrieve ciphertexts for mixing",))?;
    let pk = strand::elgamal::PublicKey::from_element(&dkg_pk.pk, &ctx);

    let seed = cfg.label(*batch, format!("shuffle_generators{mix_no}"));
    info!("Mix computing generators..");

    let hs = ctx.generators(cs.0.len() + 1, &seed);
    let shuffler = strand::shuffler::Shuffler::new(&pk, &hs, &ctx);

    info!("Mix computing shuffle..");
    let (e_primes, rs, perm) = shuffler.gen_shuffle(&cs.0);

    let label = cfg.label(*batch, format!("shuffle{mix_no}"));
    let proof = shuffler.gen_proof(&cs.0, &e_primes, &rs, &perm, &label)?;

    // FIXME removed self-verify
    // let ok = shuffler.check_proof(&proof, &cs, &e_primes, &label);
    // assert!(ok);

    let mix = Mix::new(e_primes, proof, *mix_no);
    let m = Message::mix_msg(cfg, *batch, *source_h, &mix, trustee)?;
    Ok(vec![m])
}

pub(crate) fn sign_mix<C: Ctx>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    source_h: &CiphertextsHash,
    // mix source signer
    signers_t: TrusteePosition,
    cipher_h: &CiphertextsHash,
    // mix target signer
    signert_t: TrusteePosition,
    pk_h: &PublicKeyHash,
    mix_no: &MixNumber,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>> {
    let ctx = C::default();

    let cfg = trustee.get_configuration(cfg_h)?;
    let source = if signers_t == PROTOCOL_MANAGER_INDEX {
        let cs = trustee.get_ballots(source_h, *batch, PROTOCOL_MANAGER_INDEX);
        if let Some(ballots) = cs {
            info!(
                "SignMix verifying shuffle [{} (ballots)] => [{}] ({})..",
                dbg_hash(&source_h.0),
                dbg_hash(&cipher_h.0),
                ballots.ciphertexts.0.len()
            );
            Some(ballots.ciphertexts)
        } else {
            error!("Could not retrieve ciphertexts for mixing");
            None
        }
    } else {
        let mix = trustee.get_mix(source_h, *batch, signers_t);
        if let Some(cs) = mix {
            info!(
                "SignMix verifying shuffle [{} (mix)] => [{}] ({})..",
                dbg_hash(&source_h.0),
                dbg_hash(&cipher_h.0),
                cs.ciphertexts.0.len()
            );
            Some(cs.ciphertexts)
        } else {
            error!("Could not retrieve ciphertexts for mixing");
            None
        }
    };

    let target = trustee.get_mix(cipher_h, *batch, signert_t);

    let source_cs = source.ok_or(anyhow!("Failed to retrieve source of mix to sign",))?;
    let mix = target.ok_or(anyhow!("Failed to retrieve target of mix to sign",))?;

    let mix_number = mix.mix_number;
    let dkg_pk = trustee
        .get_dkg_public_key(pk_h, 0)
        .ok_or(anyhow!("Could not retrieve public key for mixing",))?;
    let pk = strand::elgamal::PublicKey::from_element(&dkg_pk.pk, &ctx);

    let seed = cfg.label(*batch, format!("shuffle_generators{mix_no}"));
    let hs = ctx.generators(source_cs.0.len() + 1, &seed);
    let shuffler = strand::shuffler::Shuffler::new(&pk, &hs, &ctx);

    let label = cfg.label(*batch, format!("shuffle{mix_number}"));
    let ok = shuffler.check_proof(&mix.proof, &source_cs.0, &mix.ciphertexts.0, &label)?;
    info!(
        "SignMix verifying shuffle [{}] => [{}] ok = {}",
        dbg_hash(&source_h.0),
        dbg_hash(&cipher_h.0),
        ok
    );
    assert!(ok);
    let m = Message::mix_signed_msg(cfg, *batch, *source_h, *cipher_h, mix_number, trustee)?;
    Ok(vec![m])
}
