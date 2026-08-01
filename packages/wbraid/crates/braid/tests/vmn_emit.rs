// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Emit a Verificatum proof directory from a shuffle braid performed.
//!
//! This is the other direction of the interop: `vmn_interop.rs` shows braid
//! verifying Verificatum's proof, and this produces a proof for `vmnv` to check.
//! The test itself writes the directory and verifies it with braid; running
//! `vmnv -shuffle` on the output is the external step.
//!
//! Set `VMN_EMIT_DIR` to choose where to write (the directory is created).
//! Without it the test still runs, using a temporary directory, so the emitter
//! is exercised in CI even when no Java is available.

#![cfg(feature = "native")]

use std::path::PathBuf;

use braid::vmn::proof_dir::{MixerStep, ShufflingProof};
use braid::vmn::{challenges::VmnChallenges, generators::vmn_generators};
use cryptography::context::{Context, P256Ctx};
use cryptography::cryptosystem::elgamal::{Ciphertext, KeyPair};
use cryptography::zkp::shuffle::Shuffler;
use vcompat::crypto::{global_prefix, Hashfunction, PrefixParams};

const W: usize = 2;
const N: usize = 8;
const N_R: usize = 100;
const N_E: usize = 256;
const N_V: usize = 256;

/// Must match the `<pgroup>` value in the protocol info file handed to `vmnv`.
const PGROUP: &str = "ECqPGroup(P-256)::0000000002010000002\
0636f6d2e766572696669636174756d2e61726974686d2e4543715047726f757001000000\
05502d323536";

const SID: &str = "braidpoc";
const AUXSID: &str = "default";

fn rho() -> Vec<u8> {
    global_prefix(
        Hashfunction::Sha256,
        &PrefixParams {
            version: "3.1.0".into(),
            sid: SID.into(),
            auxsid: AUXSID.into(),
            n_r: N_R as u32,
            n_v: N_V as u32,
            n_e: N_E as u32,
            prg: "SHA-256".into(),
            pgroup: PGROUP.into(),
            rohash: "SHA-256".into(),
        },
    )
}

#[test]
fn emit_a_verificatum_shuffling_proof() {
    let out = match std::env::var("VMN_EMIT_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => std::env::temp_dir().join("braid_vmn_emit"),
    };
    let _ = std::fs::remove_dir_all(&out);

    // --- a shuffle performed by braid ------------------------------------
    let keypair: KeyPair<P256Ctx> = KeyPair::generate();
    let messages: Vec<[<P256Ctx as Context>::Element; W]> = (0..N)
        .map(|_| std::array::from_fn(|_| P256Ctx::random_element()))
        .collect();
    let input: Vec<Ciphertext<P256Ctx, W>> =
        messages.iter().map(|m| keypair.encrypt(m)).collect();

    let rho = rho();
    let generators = vmn_generators(Hashfunction::Sha256, &rho, N_R, N).expect("generators");
    let shuffler = Shuffler::<P256Ctx, W>::new(generators.clone(), keypair.pkey.clone());

    // A separate instance per proof: VmnChallenges caches the batching seed, so
    // it is single-use (see its type docs).
    let proving_challenges = VmnChallenges::new(Hashfunction::Sha256, rho.clone(), N_E, N_V, W);
    let (output, proof) = shuffler
        .shuffle_with(&input, &[], &proving_challenges)
        .expect("shuffle must succeed");

    // Self-check before writing: the proof must verify under the same
    // convention it was produced with.
    let verifying_challenges = VmnChallenges::new(Hashfunction::Sha256, rho, N_E, N_V, W);
    assert!(
        shuffler
            .verify_with(&input, &output, &proof, &[], &verifying_challenges)
            .expect("verification must not error"),
        "a proof produced under the Verificatum convention must verify under it"
    );

    // --- write the proof directory ---------------------------------------
    ShufflingProof::<W> {
        version: "3.1.0",
        auxsid: AUXSID,
        width: W,
        public_key: &keypair.pkey.y,
        threshold: 1,
        input: &input,
        mixers: &[MixerStep {
            output: &output,
            proof: &proof,
        }],
        // This session's protocol info declares threshold 1, so the polynomial
        // in the exponent is the single element (y). Algorithm 24 checks
        // Gamma_0 == y, so it has to be exactly this.
        polynomial_in_exponent: Some(&[keypair.pkey.y]),
    }
    .write(&out)
    .expect("write proof directory");

    // The files vmnv requires for a shuffling proof (VMNV §9.1).
    for name in [
        "version",
        "type",
        "auxsid",
        "width",
        "FullPublicKey.bt",
        "Ciphertexts.bt",
        "ShuffledCiphertexts.bt",
        "proofs/activethreshold",
        // vmnv does not read this for a shuffling proof, but VMNV §9.3 step 5
        // does, and checks Gamma_0 against the public key, so a complete
        // directory carries it.
        "proofs/PolynomialInExponent.bt",
        "proofs/Ciphertexts01.bt",
        "proofs/PermutationCommitment01.bt",
        "proofs/PoSCommitment01.bt",
        "proofs/PoSReply01.bt",
    ] {
        let path = out.join(name);
        assert!(path.is_file(), "missing {name}");
        assert!(
            std::fs::metadata(&path).unwrap().len() > 0,
            "{name} is empty"
        );
    }

    eprintln!("wrote a shuffling proof for N={N} width={W} to {}", out.display());

    // --- the emitter's own preconditions ---------------------------------
    //
    // These are checks `vmnv -shuffle` cannot make for us: it never reads the
    // polynomial, so a directory violating Algorithm 24 would pass it and be
    // rejected only by a verifier that follows the specification. Nothing
    // downstream would catch that, which is why the emitter refuses up front.
    //
    // Each case below is a distinct way Algorithm 24 would reject:
    // "attempt to read Gamma = (Gamma_0, ..., Gamma_{lambda-1}) ... if this
    // fails or if Gamma_0 != y, then reject".
    let attempt = |threshold: usize, gamma: Option<&[<P256Ctx as Context>::Element]>, mixers: &[MixerStep<W>]| {
        ShufflingProof::<W> {
            version: "3.1.0",
            auxsid: AUXSID,
            width: W,
            public_key: &keypair.pkey.y,
            threshold,
            input: &input,
            mixers,
            polynomial_in_exponent: gamma,
        }
        .write(&out.join("rejected"))
    };
    let one_mixer = [MixerStep {
        output: &output,
        proof: &proof,
    }];

    // Gamma_0 != y: the one coefficient checkable against an independent source.
    let wrong_gamma = [P256Ctx::random_element()];
    assert!(
        attempt(1, Some(&wrong_gamma), &one_mixer).is_err(),
        "a polynomial inconsistent with the public key must be refused"
    );

    // Too few entries: reading lambda of them fails, so a strict verifier rejects.
    let short_gamma = [keypair.pkey.y];
    assert!(
        attempt(2, Some(&short_gamma), &one_mixer).is_err(),
        "a polynomial shorter than the threshold must be refused"
    );

    // Too many entries, same reasoning in the other direction.
    let long_gamma = [keypair.pkey.y, P256Ctx::random_element()];
    assert!(
        attempt(1, Some(&long_gamma), &one_mixer).is_err(),
        "a polynomial longer than the threshold must be refused"
    );

    // Fewer mixers than the threshold describes a session that can never meet
    // it, since lambda_a >= lambda by construction.
    assert!(
        attempt(2, None, &one_mixer).is_err(),
        "fewer mixers than the threshold must be refused"
    );

    // And the correct shape is still accepted, so the checks above are not
    // simply rejecting everything.
    assert!(
        attempt(1, Some(&[keypair.pkey.y]), &one_mixer).is_ok(),
        "a well-formed polynomial must still be accepted"
    );
}
