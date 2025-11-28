#![allow(clippy::too_many_arguments)]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;

use super::*;

/// Computes a mix.
///
/// If this is the first mix of the chain (mix_no == 1),
/// the mix inputs come from the Ballots artifact. Other
/// wise they come from the previous Mix.
///
/// The proof of shuffle is not self verified. If the number
/// of ciphertexts is zero, the output has zero ciphertexts
/// an no proof (None).
///
/// Returns a Message of type Mix signed by the current trustee.
///
/// The shuffle is implemented in strand, as described in Haenni
/// et al., Haines; based on Wikstrom et al. The generators
/// are computed with a seed from the configuration label
/// and the mix number.
pub(crate) fn mix<C: Ctx>(
    cfg_h: &ConfigurationHash,
    batch: &BatchNumber,
    source_h: &CiphertextsHash,
    pk_h: &PublicKeyHash,
    signer_t: TrusteePosition,
    mix_no: &MixNumber,
    trustees: &TrusteeSet,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>, ProtocolError> {
    let cfg = trustee.get_configuration(cfg_h)?;
    let ctx = C::default();

    let ciphertexts = if *mix_no == 1 {
        assert_eq!(signer_t, PROTOCOL_MANAGER_INDEX);
        // Ballot ciphertexts
        let ballots = trustee
            .get_ballots(source_h, *batch, PROTOCOL_MANAGER_INDEX)
            .add_context("Mixing")?;

        info!(
            "Mix computing shuffle [{} (ballots)] ({})..",
            dbg_hash(&source_h.0),
            ballots.ciphertexts.0.len()
        );
        ballots.ciphertexts
    } else {
        // First mix ciphertexts come from ballots, second from first mix, third from second, etc.
        // mix_no is 1-based, but trustees[] is 0-based, so the previous mixer is
        // the trustee at index n - 2 (= (n - 1) - 1).
        //
        // For example, if we're on mix #2,
        // the source mix is signed by the first trustee, which is trustees[0].
        //
        // Trustees[] elements are 1-based, so trustees[mix_no - 2] - 1.
        assert_eq!(signer_t, trustees[mix_no - 2] - 1);
        let signer_t = trustees[mix_no - 2] - 1;
        let mix = trustee
            .get_mix(source_h, *batch, signer_t)
            .add_context("Mixing")?;

        info!(
            "Mix computing shuffle [{} (mix)] ({})..",
            dbg_hash(&source_h.0),
            mix.ciphertexts.0.len()
        );

        mix.ciphertexts
    };

    // Null mix
    if ciphertexts.0.len() == 0 {
        let mix = Mix::null(*mix_no);
        let m = Message::mix_msg(cfg, *batch, *source_h, &mix, trustee)?;
        return Ok(vec![m]);
    }

    let dkg_pk = trustee.get_dkg_public_key(pk_h, 0).add_context("Mixing")?;
    let pk = strand::elgamal::PublicKey::from_element(&dkg_pk.pk, &ctx);

    let seed = cfg.label(*batch, format!("shuffle_generators{mix_no}"));
    info!("Mix computing generators..");

    let hs = ctx.generators(ciphertexts.0.len() + 1, &seed)?;
    let shuffler = strand::shuffler::Shuffler::new(&pk, &ctx);

    info!("Mix computing shuffle..");
    let (e_primes, rs, perm) = shuffler.gen_shuffle(&ciphertexts.0);

    let label = cfg.label(*batch, format!("shuffle{mix_no}"));
    let proof = shuffler.gen_proof(ciphertexts.0, &e_primes, rs, hs, perm, &label)?;

    // FIXME removed self-verify
    // let ok = shuffler.check_proof(&proof, &cs, &e_primes, &label);
    // assert!(ok);

    let mix = Mix::new(e_primes, proof, *mix_no);
    let m = Message::mix_msg(cfg, *batch, *source_h, &mix, trustee)?;
    Ok(vec![m])
}

/// Verifies a mix.
///
/// If the number of ciphertexts is zero, the output ciphertexts
/// is checked to be zero and the proof is checked to be None.
///
/// Returns a Message of type MixSigned signed by this trustee.
///
/// The shuffle is implemented in strand, as described in Haenni
/// et al., Haines; based on Wikstrom et al. The generators
/// are computed with a seed from the configuration label
/// and the mix number.
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
) -> Result<Vec<Message>, ProtocolError> {
    let ctx = C::default();

    let cfg = trustee.get_configuration(cfg_h)?;
    let source_cs = if signers_t == PROTOCOL_MANAGER_INDEX {
        let ballots = trustee
            .get_ballots(source_h, *batch, PROTOCOL_MANAGER_INDEX)
            .add_context("Signing mix")?;

        info!(
            "SignMix verifying shuffle [{} (ballots)] => [{}] ({})..",
            dbg_hash(&source_h.0),
            dbg_hash(&cipher_h.0),
            ballots.ciphertexts.0.len()
        );
        ballots.ciphertexts
    } else {
        let mix = trustee
            .get_mix(source_h, *batch, signers_t)
            .add_context("Signing mix")?;

        info!(
            "SignMix verifying shuffle [{} (mix)] => [{}] ({})..",
            dbg_hash(&source_h.0),
            dbg_hash(&cipher_h.0),
            mix.ciphertexts.0.len()
        );
        mix.ciphertexts
    };

    let target = trustee.get_mix(cipher_h, *batch, signert_t);
    let mix = target.add_context("Signing mix")?;

    let mix_number = mix.mix_number;

    // Null mix
    if source_cs.0.len() == 0 {
        if (mix.ciphertexts.0.len() != 0) || mix.proof.is_some() {
            return Err(ProtocolError::VerificationError(format!(
                "A null mix should have no outout ciphertexts or proof"
            )));
        }

        let m = Message::mix_signed_msg(cfg, *batch, *source_h, *cipher_h, mix_number, trustee)?;
        return Ok(vec![m]);
    }

    let Some(proof) = mix.proof else {
        return Err(ProtocolError::VerificationError(format!(
            "Mix cannot be null if there are source ciphertexts"
        )));
    };

    let dkg_pk = trustee
        .get_dkg_public_key(pk_h, 0)
        .add_context("Signing mix")?;
    let pk = strand::elgamal::PublicKey::from_element(&dkg_pk.pk, &ctx);

    let seed = cfg.label(*batch, format!("shuffle_generators{mix_no}"));
    let hs = ctx.generators(source_cs.0.len() + 1, &seed)?;
    let shuffler = strand::shuffler::Shuffler::new(&pk, &ctx);

    let label = cfg.label(*batch, format!("shuffle{mix_number}"));
    let ok = shuffler.check_proof(&proof, source_cs.0, mix.ciphertexts.0, hs, &label)?;
    info!(
        "SignMix shuffle verification [{}] => [{}] ok = {}",
        dbg_hash(&source_h.0),
        dbg_hash(&cipher_h.0),
        ok
    );

    if !ok {
        return Err(ProtocolError::VerificationError(format!(
            "Mix verification failed"
        )));
    }

    let m = Message::mix_signed_msg(cfg, *batch, *source_h, *cipher_h, mix_number, trustee)?;
    Ok(vec![m])
}
