// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Run Verificatum's `vmnv` against a proof braid produced.
//!
//! `vmn_interop.rs` covers the direction that needs no external tooling — braid
//! verifying Verificatum's proof. This is the other direction, and it is the
//! claim the whole exercise rests on: **an independently written verifier, in a
//! different language, accepts braid's output.** Without this test that claim
//! would only be reproducible by hand.
//!
//! `#[ignore]` because it shells out to a JVM, following the convention of the
//! HTTP protocol tests that need a live b4. Run with:
//!
//! ```text
//! VMNV_JAR_DIR=.../verificatum \
//! VMNV_PROTINFO=.../protInfo.xml \
//! VMNV_RANDOM_SOURCE=.../random_source \
//! VMNV_RANDOM_SEED=.../random_seed \
//! cargo test -p braid --test vmn_verifier -- --ignored --nocapture
//! ```
//!
//! `VMNV_JAR_DIR` must contain `verificatum-vmn-3.1.0.jar` and
//! `verificatum-vcr-3.1.0.jar`; the random source/seed are the files
//! `vog -rndinit` writes. `VMNV_JAVA` overrides the `java` binary.
//!
//! The session parameters below must match `VMNV_PROTINFO`, because the global
//! prefix rho is derived from them and the verifier recomputes it.
//!
//! # Never check `vmnv`'s exit code on its own
//!
//! `vmnv` exits 0 on shuffling proofs it has itself rejected — see
//! [`vmnv_exit_code_alone_is_not_sufficient`] for the root cause. Ask
//! [`vmnv_accepts`] instead, and if this interop later grows a CI job or tooling,
//! that predicate is what it should use.
//!
//! Full `-mix` happens to reject these cases via its downstream plaintext
//! comparison, so [`vmnv_accepts_a_braid_mixing_proof`] can assert on the exit
//! code. **`-mix -nodec` cannot** — it skips that comparison and is affected
//! exactly like `-shuffle`, so it is not a safe way to check the mixing phase
//! alone.

#![cfg(feature = "native")]

use std::path::PathBuf;
use std::process::Command;

use braid::vmn::proof_dir::{MixerStep, ShufflingProof};
use braid::vmn::{challenges::VmnChallenges, generators::vmn_generators};
use cryptography::context::{Context, P256Ctx};
use cryptography::groups::p256::element::P256Element;
use cryptography::cryptosystem::elgamal::{Ciphertext, KeyPair};
use cryptography::zkp::shuffle::Shuffler;
use vcompat::crypto::{global_prefix, Hashfunction, PrefixParams};

const W: usize = 2;
const N: usize = 8;
const N_R: usize = 100;
const N_E: usize = 256;
const N_V: usize = 256;
const SID: &str = "braidpoc";
const AUXSID: &str = "default";

const PGROUP: &str = "ECqPGroup(P-256)::0000000002010000002\
0636f6d2e766572696669636174756d2e61726974686d2e4543715047726f757001000000\
05502d323536";

const VERIFY_TOOL: &str = "com.verificatum.protocol.mixnet.MixNetElGamalVerifyFiatShamirTool";

struct Env {
    java: String,
    classpath: String,
    protinfo: PathBuf,
    random_source: PathBuf,
    random_seed: PathBuf,
}

/// Collect the external configuration, or `None` if this environment cannot run
/// the verifier.
fn env() -> Option<Env> {
    let jar_dir = PathBuf::from(std::env::var("VMNV_JAR_DIR").ok()?);
    let vmn = jar_dir.join("verificatum-vmn/verificatum-vmn-3.1.0.jar");
    let vcr = jar_dir.join("verificatum-vcr/verificatum-vcr-3.1.0.jar");
    if !vmn.is_file() || !vcr.is_file() {
        eprintln!("skipping: jars not found under {}", jar_dir.display());
        return None;
    }
    let separator = if cfg!(windows) { ";" } else { ":" };

    Some(Env {
        java: std::env::var("VMNV_JAVA").unwrap_or_else(|_| "java".to_string()),
        classpath: format!("{}{separator}{}", vmn.display(), vcr.display()),
        protinfo: match std::env::var("VMNV_PROTINFO") {
            Ok(p) => PathBuf::from(p),
            // The in-repo corpus ships the matching protocol info file.
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../testdata/verificatum/protInfo.xml"),
        },
        random_source: PathBuf::from(std::env::var("VMNV_RANDOM_SOURCE").ok()?),
        random_seed: PathBuf::from(std::env::var("VMNV_RANDOM_SEED").ok()?),
    })
}

/// `vmnv -shuffle <protInfo> <dir>`; returns the exit code and combined output.
///
/// `verbose` adds `-v`. It matters more than it looks: `vmnv` reports some
/// failures only when verbose, so a non-verbose run can be silent about a proof
/// it rejected internally (see `vmnv_is_silent_about_a_failed_shuffle`).
fn run_vmnv(env: &Env, dir: &PathBuf, verbose: bool) -> (i32, String) {
    run_vmnv_mode(env, dir, "-shuffle", verbose)
}

/// As [`run_vmnv`], with the session type selected by `mode` (`-shuffle` or
/// `-mix`).
fn run_vmnv_mode(env: &Env, dir: &PathBuf, mode: &str, verbose: bool) -> (i32, String) {
    let seed = private_seed(env);

    let mut command = Command::new(&env.java);
    command
        .arg("-cp")
        .arg(&env.classpath)
        .arg(VERIFY_TOOL)
        // The launcher script passes these three ahead of the real arguments.
        .arg("vmnv")
        .arg(&env.random_source)
        .arg(&seed)
        .arg(mode)
        .arg("-wd")
        .arg(private_name("wd"));
    if verbose {
        command.arg("-v");
    }
    let output = command
        .arg("-auxsid")
        .arg(AUXSID)
        .arg("-width")
        .arg(W.to_string())
        .arg(&env.protinfo)
        .arg(dir)
        .output()
        .expect("failed to launch java");
    let _ = std::fs::remove_file(&seed);

    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code().unwrap_or(-1), text)
}

/// A name unique to this invocation, for the two pieces of mutable state `vmnv`
/// would otherwise share between concurrent runs.
///
/// **`-wd` is the one that actually bites.** Verificatum spools large integer
/// arrays into a working directory under `/tmp/com.verificatum`, and without
/// `-wd` every process picks the *same* one and deletes it on exit, so parallel
/// runs kill each other with `File not found!` or `Unable to delete storage
/// directory!` part-way through a proof. It must be a relative name: `TempFile`
/// treats a path as absolute only if it starts with `/`, so a Windows path would
/// be appended to the default root rather than replacing it.
fn private_name(kind: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    format!(
        "braid_vmnv_{kind}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// A private copy of the seed file for one `vmnv` invocation.
///
/// `vmnv` rewrites its seed on every run — that is what a seeded PRG source does
/// — so this is the second file concurrent runs would share. Unlike the working
/// directory it has not been observed to cause a failure, but sharing mutable
/// state across parallel processes to save a file copy is not a trade worth
/// making. The random *source* file is only read, and stays shared.
fn private_seed(env: &Env) -> PathBuf {
    let path = std::env::temp_dir().join(private_name("seed"));
    std::fs::copy(&env.random_seed, &path).expect("copy the vmnv seed file");
    path
}

/// Did `vmnv` accept this proof? **Use this rather than the exit code.**
///
/// `vmnv`'s exit status is not a sound accept/reject signal for a shuffling
/// proof: it exits 0 on proofs it has itself rejected (see
/// [`vmnv_exit_code_alone_is_not_sufficient`]). A correct check therefore has to
/// require both a zero exit *and* positive confirmation that the shuffle
/// verification ran to completion.
///
/// This matters beyond documenting someone else's bug. If braid's emitter ever
/// drifts — a change to `VmnChallenges`, the generator derivation, or the byte
/// encoding — the proofs become invalid, and `vmnv` would report that by exiting
/// 0. Anything checking only the exit status would see a passing test.
/// [`vmnv_would_catch_emitter_drift`] demonstrates that this predicate does not.
fn vmnv_accepts(env: &Env, dir: &PathBuf) -> bool {
    let (code, output) = run_vmnv(env, dir, true);
    code == 0 && output.contains("Verify proof of shuffle... done.")
}

/// Rewrite a proof directory to claim the output is the input, i.e. that nothing
/// was shuffled. The proof is then invalid for that statement.
fn claim_no_shuffling_happened(dir: &PathBuf) {
    let input = std::fs::read(dir.join("Ciphertexts.bt")).unwrap();
    std::fs::write(dir.join("ShuffledCiphertexts.bt"), &input).unwrap();
    std::fs::write(dir.join("proofs/Ciphertexts01.bt"), &input).unwrap();
}

/// Produce a shuffle and write it as a Verificatum proof directory.
fn emit(dir: &PathBuf) {
    emit_with_drift(dir, false);
}

/// As [`emit`], but `drift` perturbs the global prefix to stand in for a
/// regression in braid's transcript layer — the realistic way this interop
/// breaks. The resulting proof is well-formed but does not verify.
fn emit_with_drift(dir: &PathBuf, drift: bool) {
    let _ = std::fs::remove_dir_all(dir);

    let mut rho = global_prefix(
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
    );
    if drift {
        rho[0] ^= 0x01;
    }

    let keypair: KeyPair<P256Ctx> = KeyPair::generate();
    let input: Vec<Ciphertext<P256Ctx, W>> = (0..N)
        .map(|_| {
            let m: [<P256Ctx as Context>::Element; W] =
                std::array::from_fn(|_| P256Ctx::random_element());
            keypair.encrypt(&m)
        })
        .collect();

    let generators = vmn_generators(Hashfunction::Sha256, &rho, N_R, N).expect("generators");
    let shuffler = Shuffler::<P256Ctx, W>::new(generators, keypair.pkey.clone());
    let challenges = VmnChallenges::new(Hashfunction::Sha256, rho, N_E, N_V, W);
    let (output, proof) = shuffler
        .shuffle_with(&input, &[], &challenges)
        .expect("shuffle");

    ShufflingProof::<W> {
        version: "3.1.0",
        auxsid: AUXSID,
        width: W,
        threshold: 1,
        public_key: &keypair.pkey.y,
        input: &input,
        mixers: &[MixerStep { output: &output, proof: &proof }],
        polynomial_in_exponent: Some(&polynomial_in_exponent(&keypair.pkey.y, 1)),
    }
    .write(dir)
    .expect("write proof directory");
}

/// **The headline result**: unmodified `vmnv` accepts a proof braid produced.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_accepts_a_braid_shuffle_proof() {
    let Some(env) = env() else {
        eprintln!("skipping: VMNV_* environment not configured");
        return;
    };

    let dir = std::env::temp_dir().join("braid_vmnv_accept");
    emit(&dir);

    assert!(
        vmnv_accepts(&env, &dir),
        "vmnv must accept a proof braid produced"
    );
}

/// A polynomial in the exponent `Γ = (Γ_0, ..., Γ_{λ-1})` with `Γ_0 = y`.
///
/// `vmnv` does not read this for a shuffling proof, but VMNV §9.3 step 5 and
/// §9.1 both say a proof directory contains it, and VMN's own prover writes it
/// even for shuffling sessions. Emitting it keeps braid's proofs acceptable to a
/// verifier written strictly to the specification, rather than only to one that
/// shares `vmnv`'s leniency — which is the entire point of the exercise.
///
/// The higher coefficients are arbitrary group elements. Every element of a
/// prime-order group is `g^γ` for some `γ`, so this is a well-formed degree
/// `λ-1` polynomial whose constant term is the real secret; a shuffling session
/// never decrypts, so the coefficients above it are never used.
fn polynomial_in_exponent(y: &P256Element, threshold: usize) -> Vec<P256Element> {
    let mut gamma = Vec::with_capacity(threshold);
    gamma.push(*y);
    for _ in 1..threshold {
        gamma.push(P256Ctx::random_element());
    }
    gamma
}

/// A chain of `parties` mixers, each shuffling the previous output, written as
/// one multi-party shuffling proof.
///
/// The independent generators are a **session-level** value derived once from
/// the prefix; every mixer shares them. Only the per-mixer statement differs,
/// since each batching seed commits to that mixer's own permutation commitment
/// and its input/output pair.
fn emit_chain(dir: &PathBuf, parties: usize) {
    let _ = std::fs::remove_dir_all(dir);

    let rho = global_prefix(
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
    );

    let keypair: KeyPair<P256Ctx> = KeyPair::generate();
    let input: Vec<Ciphertext<P256Ctx, W>> = (0..N)
        .map(|_| {
            let m: [<P256Ctx as Context>::Element; W] =
                std::array::from_fn(|_| P256Ctx::random_element());
            keypair.encrypt(&m)
        })
        .collect();

    let generators = vmn_generators(Hashfunction::Sha256, &rho, N_R, N).expect("generators");
    let shuffler = Shuffler::<P256Ctx, W>::new(generators, keypair.pkey.clone());

    // Run the chain, keeping each mixer's output and proof.
    let mut current = input.clone();
    let mut outputs = Vec::with_capacity(parties);
    let mut proofs = Vec::with_capacity(parties);
    for _ in 0..parties {
        let challenges = VmnChallenges::new(Hashfunction::Sha256, rho.clone(), N_E, N_V, W);
        let (output, proof) = shuffler
            .shuffle_with(&current, &[], &challenges)
            .expect("shuffle");
        current = output.clone();
        outputs.push(output);
        proofs.push(proof);
    }

    let gamma = polynomial_in_exponent(&keypair.pkey.y, 3);

    let mixers: Vec<MixerStep<W>> = outputs
        .iter()
        .zip(proofs.iter())
        .map(|(output, proof)| MixerStep { output, proof })
        .collect();

    ShufflingProof::<W> {
        version: "3.1.0",
        auxsid: AUXSID,
        width: W,
        threshold: 3,
        public_key: &keypair.pkey.y,
        input: &input,
        mixers: &mixers,
        polynomial_in_exponent: Some(&gamma),
    }
    .write(dir)
    .expect("write proof directory");
}

/// **Multi-party interop**: `vmnv` accepts a chain of three mixers braid ran.
///
/// Needs a protocol info file declaring three parties, so it is skipped unless
/// `VMNV_PROTINFO_MULTI` points at one (`testdata/verificatum/protInfo-3party.xml`
/// is the shipped one). The session parameters are otherwise identical, which is
/// why the prefix is unchanged: rho commits to the widths, hashes and group, but
/// not to the party count.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_accepts_a_three_party_chain() {
    let Some(mut env) = env() else {
        eprintln!("skipping: VMNV_* environment not configured");
        return;
    };
    env.protinfo = match std::env::var("VMNV_PROTINFO_MULTI") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../testdata/verificatum/protInfo-3party.xml"),
    };
    if !env.protinfo.is_file() {
        eprintln!("skipping: no three-party protocol info file");
        return;
    }

    let dir = std::env::temp_dir().join("braid_vmnv_chain");
    emit_chain(&dir, 3);

    let (code, output) = run_vmnv(&env, &dir, true);
    eprintln!("{output}");
    assert_eq!(code, 0, "vmnv must accept the chain");

    // Every mixer's proof must have been verified, not just the first.
    for party in 1..=3 {
        assert!(
            output.contains(&format!("Verify shuffle of Party {party}.")),
            "vmnv must verify party {party}; got:\n{output}"
        );
    }
    assert_eq!(
        output.matches("Verify proof of shuffle... done.").count(),
        3,
        "all three shuffle proofs must verify; got:\n{output}"
    );
}

/// Corrupting any single mixer's proof must sink the whole chain, so a chain is
/// not accepted on the strength of its other members.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_rejects_a_chain_with_one_bad_mixer() {
    let Some(mut env) = env() else {
        eprintln!("skipping: VMNV_* environment not configured");
        return;
    };
    env.protinfo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/verificatum/protInfo-3party.xml");
    if !env.protinfo.is_file() {
        eprintln!("skipping: no three-party protocol info file");
        return;
    }

    for party in 1..=3 {
        let dir = std::env::temp_dir().join("braid_vmnv_chain_bad");
        emit_chain(&dir, 3);

        let path = dir.join(format!("proofs/PoSReply{party:02}.bt"));
        let mut bytes = std::fs::read(&path).expect("read reply");
        bytes[200] ^= 0xFF;
        std::fs::write(&path, &bytes).expect("write tampered reply");

        let (_, output) = run_vmnv(&env, &dir, true);
        assert!(
            output.matches("Verify proof of shuffle... done.").count() < 3,
            "corrupting party {party} must break the chain; got:\n{output}"
        );
        eprintln!("ok: party {party} corrupted -> chain not fully verified");
    }
}

/// Guards the interop against a silent regression in **our** code.
///
/// This is the failure mode that the `vmnv` exit-code defect makes dangerous for
/// this project. If braid's transcript layer drifts, the emitted proofs stop
/// verifying — and `vmnv` reports that by exiting **0**, so a CI job checking
/// only the exit status would stay green while the interop was broken.
///
/// Here a deliberately perturbed prefix stands in for such a regression. The
/// exit code is asserted to be 0, confirming the trap is real, and
/// [`vmnv_accepts`] is asserted to reject anyway — which is what makes the
/// positive test above trustworthy.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_would_catch_emitter_drift() {
    let Some(env) = env() else {
        eprintln!("skipping: VMNV_* environment not configured");
        return;
    };

    let dir = std::env::temp_dir().join("braid_vmnv_drift");
    emit_with_drift(&dir, true);

    let (code, _) = run_vmnv(&env, &dir, true);
    assert_eq!(
        code, 0,
        "the trap this test exists for: vmnv exits 0 on the broken proof"
    );
    assert!(
        !vmnv_accepts(&env, &dir),
        "but our acceptance check must still reject it"
    );
}

/// The result above is only meaningful if `vmnv` would reject a bad proof, so
/// corrupt each proof artifact in turn and require a non-zero exit.
///
/// Bytes are flipped by XOR rather than overwritten with a fixed value: an
/// earlier version of this check wrote `0x01` over a byte that already held
/// `0x01`, so the "tampering" was a no-op and appeared to show `vmnv` accepting
/// a corrupted proof.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_rejects_tampered_braid_proofs() {
    let Some(env) = env() else {
        eprintln!("skipping: VMNV_* environment not configured");
        return;
    };

    for (name, offset) in [
        ("proofs/PoSReply01.bt", 200usize),
        ("proofs/PoSCommitment01.bt", 100),
        ("proofs/PermutationCommitment01.bt", 50),
        ("Ciphertexts.bt", 60),
    ] {
        let dir = std::env::temp_dir().join("braid_vmnv_reject");
        emit(&dir);

        let path = dir.join(name);
        let mut bytes = std::fs::read(&path).expect("read artifact");
        assert!(offset < bytes.len(), "{name} is shorter than {offset}");
        bytes[offset] ^= 0xFF;
        std::fs::write(&path, &bytes).expect("write tampered artifact");

        let (code, _) = run_vmnv(&env, &dir, true);
        assert_ne!(code, 0, "vmnv must reject a proof with {name} corrupted");
        assert!(
            !vmnv_accepts(&env, &dir),
            "and our acceptance check must agree for {name}"
        );
        eprintln!("ok: {name} corrupted -> vmnv exit {code}");
    }
}

/// A defect in `vmnv`, pinned here so the behaviour is not mistaken for ours and
/// so a fix upstream is noticed.
///
/// Claiming the output is the input makes the shuffle proof invalid. `vmnv`
/// detects that and says so under `-v` — `Verify proof of shuffle... failed.`
/// and `Too few proofs are valid! (0)` — and then **exits 0**.
///
/// The cause is visible in `MixNetElGamalVerifyFiatShamirSession`, which reaches
/// the right conclusion and routes it to the wrong handler:
///
/// ```java
/// if (validProofs < v.threshold) {
///     v.failInfo("Too few proofs are valid! (" + validProofs + ")");
/// }
/// ```
///
/// `failInfo` only prints, and only when verbose. Its sibling `failStop` throws
/// `ProtocolError` and halts. `validProofs < threshold` is exactly VMNV §2.3's
/// reject condition ("If less than λ proofs are valid, then reject"), so the
/// condition is evaluated correctly and then not enforced.
///
/// Callers must therefore **not rely on `vmnv`'s exit code alone** for a
/// shuffling proof.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_exit_code_alone_is_not_sufficient() {
    let Some(env) = env() else {
        eprintln!("skipping: VMNV_* environment not configured");
        return;
    };

    let dir = std::env::temp_dir().join("braid_vmnv_identity");
    emit(&dir);
    claim_no_shuffling_happened(&dir);

    let (code, output) = run_vmnv(&env, &dir, true);
    assert!(
        output.contains("Verify proof of shuffle... failed."),
        "vmnv should report the shuffle proof as failed, got:\n{output}"
    );
    assert!(
        output.contains("Too few proofs are valid!"),
        "vmnv should report too few valid proofs, got:\n{output}"
    );
    assert_eq!(
        code, 0,
        "documenting vmnv's actual behaviour: it reports the failure but still exits 0"
    );
    assert!(
        !vmnv_accepts(&env, &dir),
        "our acceptance check must reject it despite the zero exit"
    );
}

/// The same defect without `-v`, which is the dangerous shape of it.
///
/// `failInfo` prints only when verbose, so a non-verbose run of a shuffling
/// proof that `vmnv` internally rejected produces **no output at all and exits
/// 0** — indistinguishable from success. A shuffling session is VMN's documented
/// mode for re-randomising without decrypting (VMNV §2.4), so accepting one in
/// which no mixing occurred means accepting a mix-net that provided no privacy.
///
/// If this assertion ever fails, `vmnv` has been fixed and the interop notes in
/// `VERIFICATUM.md` should be revisited.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_is_silent_about_a_failed_shuffle() {
    let Some(env) = env() else {
        eprintln!("skipping: VMNV_* environment not configured");
        return;
    };

    let dir = std::env::temp_dir().join("braid_vmnv_silent");
    emit(&dir);
    claim_no_shuffling_happened(&dir);

    let (code, output) = run_vmnv(&env, &dir, false);
    assert_eq!(code, 0, "vmnv exits 0 on a shuffle proof it rejected");
    assert!(
        output.trim().is_empty(),
        "and says nothing about it without -v; got:\n{output}"
    );
}

/// Emit a complete `type = mixing` proof: a real DKG, a chain of shuffles, and
/// threshold decryption with the batched proof.
///
/// `K` is the party count `k` and `T` the threshold λ, which must match the
/// protocol info file's `<nopart>` and `<thres>`. `delta` names the 1-based
/// indices of the λ parties that decrypt; any party not listed contributes the
/// all-identity factor array and the identity commitment/zero reply that
/// Verificatum records for an absent contribution.
///
/// The chain runs `T` mixers, since `λ_a ≥ λ` and braid mixes with exactly its
/// selected trustees.
fn emit_mixing<const K: usize, const T: usize>(dir: &PathBuf, delta: &[usize]) {
    use braid::vmn::decrypt::{self, batch, inactive_proof, prove_decryption};
    use braid::vmn::encode;
    use braid::vmn::proof_dir::{DecryptingParty, MixingProof};
    use cryptography::cryptosystem::elgamal::PublicKey;
    use cryptography::dkgd::dealer::{Dealer, VerifiableShare};
    use cryptography::dkgd::recipient::{ParticipantPosition, Recipient};
    use cryptography::groups::p256::scalar::P256Scalar;
    use cryptography::traits::groups::{GroupElement, GroupScalar};
    use vcompat::bytetree::ByteTree;
    use vcompat::crypto::{dec_challenge, dec_seed, Prg};

    assert_eq!(delta.len(), T, "exactly lambda parties may decrypt");
    let _ = std::fs::remove_dir_all(dir);

    let rho = global_prefix(
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
    );

    // --- the distributed key generation ---------------------------------
    let dealers: Vec<Dealer<P256Ctx, T, K>> = (0..K).map(|_| Dealer::generate()).collect();
    let dealt: Vec<_> = dealers.iter().map(|d| d.get_verifiable_shares()).collect();

    let gamma = decrypt::polynomial_in_exponent(
        &dealt
            .iter()
            .map(|s| s.checking_values.to_vec())
            .collect::<Vec<_>>(),
    )
    .expect("polynomial in the exponent");

    // Each party verifies every dealer's contribution and keeps its share.
    let mut secrets = Vec::with_capacity(K);
    let mut joint_key = None;
    for party in 1..=K {
        let shares: [VerifiableShare<P256Ctx, T>; K] = std::array::from_fn(|d| {
            VerifiableShare::new(
                dealt[d].shares[party - 1].clone(),
                dealt[d].checking_values.clone(),
            )
        });
        let (y, _vk, x_l) = Recipient::<P256Ctx, T, K>::verify_shares(
            &ParticipantPosition::from_usize(party),
            &shares,
        )
        .expect("shares must verify");
        joint_key = Some(y);
        secrets.push(x_l);
    }
    let y = joint_key.expect("a joint public key");
    assert!(gamma[0].equals(&y), "Gamma_0 must be the joint public key");

    // --- the shuffle chain ------------------------------------------------
    let pk = PublicKey::<P256Ctx>::new(y);
    let input: Vec<Ciphertext<P256Ctx, W>> = (0..N)
        .map(|_| {
            let m: [<P256Ctx as Context>::Element; W] =
                std::array::from_fn(|_| P256Ctx::random_element());
            pk.encrypt(&m)
        })
        .collect();

    let generators = vmn_generators(Hashfunction::Sha256, &rho, N_R, N).expect("generators");
    let shuffler = Shuffler::<P256Ctx, W>::new(generators, pk.clone());

    let mut current = input.clone();
    let mut outputs = Vec::with_capacity(T);
    let mut shuffle_proofs = Vec::with_capacity(T);
    for _ in 0..T {
        let challenges = VmnChallenges::new(Hashfunction::Sha256, rho.clone(), N_E, N_V, W);
        let (output, proof) = shuffler
            .shuffle_with(&current, &[], &challenges)
            .expect("shuffle");
        current = output.clone();
        outputs.push(output);
        shuffle_proofs.push(proof);
    }
    let mixed = outputs.last().expect("a non-empty chain").clone();

    // --- decryption factors, in Verificatum's convention -------------------
    // A participant scales its share by 1/alpha once: the factors use the
    // negation of that scalar and the proof reply uses it directly. A
    // non-participant publishes an all-identity array of the same shape, which
    // it must, since the verifier reads a file for every party.
    let inv_alpha = decrypt::inverse_alpha(K).expect("1/alpha");
    let u: Vec<[P256Element; W]> = mixed.iter().map(|c| c.0[0]).collect();

    let scaled: Vec<Option<P256Scalar>> = (1..=K)
        .map(|party| delta.contains(&party).then(|| secrets[party - 1].mul(&inv_alpha)))
        .collect();
    let factors: Vec<Vec<[P256Element; W]>> = scaled
        .iter()
        .map(|z| match z {
            Some(z) => {
                let exponent = z.neg();
                u.iter()
                    .map(|ui| std::array::from_fn(|w| ui[w].exp(&exponent)))
                    .collect()
            }
            None => decrypt::inactive_factors::<W>(N),
        })
        .collect();

    // --- the batched proof transcript --------------------------------------
    let factor_trees: Vec<ByteTree> = factors
        .iter()
        .map(|f| encode::component_array_to_tree(f).expect("encode factors"))
        .collect();
    let seed = dec_seed(
        Hashfunction::Sha256,
        &rho,
        &encode::element_to_tree(&P256Element::generator()).expect("encode g"),
        &encode::ciphertexts_to_tree(&mixed).expect("encode ciphertexts"),
        &encode::elements_to_tree(&gamma).expect("encode gamma"),
        &factor_trees,
    );

    // One n_e-bit batching exponent per ciphertext, as in the shuffle.
    let component = N_E.div_ceil(8);
    let stream = Prg::new(Hashfunction::Sha256, &seed).generate(component * N);
    let e: Vec<P256Scalar> = stream.chunks(component).map(scalar_from).collect();
    let a = batch(&u, &e).expect("batch the first components");

    // Commitments do not depend on the challenge, so they are fixed first and
    // then hashed into it -- including the non-participants', which is why
    // their placeholder values are not free to choose.
    let mut rng = P256Ctx::get_rng();
    let zero = P256Scalar::zero();
    let randomizers: Vec<Option<P256Scalar>> = scaled
        .iter()
        .map(|z| z.as_ref().map(|_| P256Scalar::random(&mut rng)))
        .collect();
    let commitments: Vec<_> = randomizers
        .iter()
        .map(|r| match r {
            Some(r) => prove_decryption::<W>(&zero, &a, &zero, r),
            None => inactive_proof::<W>(),
        })
        .collect();
    let commitment_trees: Vec<ByteTree> = commitments
        .iter()
        .map(|c| {
            ByteTree::node(vec![
                encode::element_to_tree(&c.y_prime).expect("encode y'"),
                encode::elements_to_tree(&c.b_prime).expect("encode B'"),
            ])
        })
        .collect();

    let v = scalar_from(&dec_challenge(
        Hashfunction::Sha256,
        N_V,
        &rho,
        &seed,
        &commitment_trees,
    ));
    let proofs: Vec<_> = (0..K)
        .map(|l| match (&scaled[l], &randomizers[l]) {
            (Some(z), Some(r)) => prove_decryption::<W>(z, &a, &v, r),
            _ => inactive_proof::<W>(),
        })
        .collect();

    // --- the plaintexts -----------------------------------------------------
    let alpha_c: Vec<P256Scalar> =
        vcompat::lagrange::p256_modified_lagrange_coefficients(delta, K)
            .into_iter()
            .map(|(negative, magnitude)| {
                let s = P256Scalar::from_bytes_reduced(&magnitude);
                if negative {
                    s.neg()
                } else {
                    s
                }
            })
            .collect();
    let plaintexts: Vec<[P256Element; W]> = (0..N)
        .map(|i| {
            let mut combined: [P256Element; W] = std::array::from_fn(|_| P256Element::one());
            for (position, &party) in delta.iter().enumerate() {
                for w in 0..W {
                    let contribution = factors[party - 1][i][w].exp(&alpha_c[position]);
                    combined[w] = combined[w].mul(&contribution);
                }
            }
            std::array::from_fn(|w| mixed[i].0[1][w].mul(&combined[w]))
        })
        .collect();

    // --- write it out -------------------------------------------------------
    let mixers: Vec<MixerStep<W>> = outputs
        .iter()
        .zip(shuffle_proofs.iter())
        .map(|(output, proof)| MixerStep { output, proof })
        .collect();
    let parties: Vec<DecryptingParty<W>> = (0..K)
        .map(|l| DecryptingParty {
            factors: &factors[l],
            proof: &proofs[l],
            participated: delta.contains(&(l + 1)),
        })
        .collect();

    MixingProof::<W> {
        shuffle: ShufflingProof {
            version: "3.1.0",
            auxsid: AUXSID,
            width: W,
            threshold: T,
            public_key: &y,
            input: &input,
            mixers: &mixers,
            polynomial_in_exponent: Some(&gamma),
        },
        plaintexts: &plaintexts,
        parties: &parties,
    }
    .write(dir)
    .expect("write the mixing proof");
}

/// Interpret a big-endian byte string as a scalar, reducing modulo the group
/// order.
///
/// Verificatum reads these as unbounded non-negative integers and exponentiates
/// by them, which is the same thing in a group of prime order.
fn scalar_from(bytes: &[u8]) -> cryptography::groups::p256::scalar::P256Scalar {
    use cryptography::groups::p256::scalar::P256Scalar;
    assert!(bytes.len() <= 32, "value wider than a P-256 scalar");
    let mut fixed = [0u8; 32];
    fixed[32 - bytes.len()..].copy_from_slice(bytes);
    P256Scalar::from_bytes_reduced(&fixed)
}

/// Assert that `vmnv -mix` verified every phase, and say which one failed if not.
fn assert_mix_verified(output: &str, mixers: usize) {
    assert!(
        output.contains("Verify combined proof of decryption... done."),
        "the batched decryption proof must verify; got:\n{output}"
    );
    assert!(
        output.contains("Match computed plaintexts with plaintexts... done."),
        "the plaintexts must match the combined factors; got:\n{output}"
    );
    assert_eq!(
        output.matches("Verify proof of shuffle... done.").count(),
        mixers,
        "every mixer's shuffle must verify too; got:\n{output}"
    );
}

/// **The decryption result**: unmodified `vmnv -mix` accepts a full braid
/// session — a real DKG, a chain of shuffles, and threshold decryption.
///
/// Run at `k = λ = 3`, so every party decrypts and the inactive path is not
/// involved. That isolates what this test settles — where `α` belongs in the
/// proof — from the separate question of what a non-participant publishes,
/// which [`vmnv_accepts_a_mixing_proof_with_an_inactive_party`] covers.
///
/// Nothing else here is degenerate: `α = lcm(1,2,3)² = 36` and the modified
/// Lagrange coefficients over `{1,2,3}` are `108`, `−108` and `36`, none of them
/// the identity the single-party corpus collapses to.
///
/// Unlike `-shuffle`, `-mix`'s exit code is a sound signal, because the
/// plaintext comparison downstream uses `failStop` — but the transcript is
/// asserted too, since that is the part which does not depend on someone else's
/// error handling.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_accepts_a_braid_mixing_proof() {
    let Some(mut env) = env() else {
        eprintln!("skipping: VMNV_* environment not configured");
        return;
    };
    env.protinfo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/verificatum/protInfo-3party.xml");
    if !env.protinfo.is_file() {
        eprintln!("skipping: no three-party protocol info file");
        return;
    }

    let dir = std::env::temp_dir().join("braid_vmnv_mix");
    emit_mixing::<3, 3>(&dir, &[1, 2, 3]);

    let (code, output) = run_vmnv_mode(&env, &dir, "-mix", true);
    eprintln!("{output}");
    assert_eq!(code, 0, "vmnv -mix must accept a braid mixing proof");
    assert_mix_verified(&output, 3);
}

/// The same at `k = 3`, `λ = 2`, with **party 2 taking no part** in decryption.
///
/// braid's model differs from Verificatum's here: only the trustees selected for
/// the mix produce decryption factors at all, whereas VMN expects a file from
/// every party and names Δ separately in `CorrectIndices.bt`. The emitter
/// bridges that with an all-identity factor array, the identity commitment and a
/// zero reply — the values `DistrElGamalSessionBasic` itself falls back to.
///
/// Those placeholders are load-bearing rather than arbitrary: **every** party's
/// commitment is hashed into the decryption challenge, so a wrong one would move
/// `v` and break the two real proofs. This test is what confirms them.
///
/// Δ is `{1, 3}` rather than `{1, 2}` so the gap is in the middle, where an
/// off-by-one in party indexing shows up. The modified Lagrange coefficients are
/// then `54` and `−18` — note `α` is still `lcm(1,2,3)² = 36`, a function of `k`
/// and not of the threshold.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_accepts_a_mixing_proof_with_an_inactive_party() {
    let Some(mut env) = env() else {
        eprintln!("skipping: VMNV_* environment not configured");
        return;
    };
    env.protinfo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/verificatum/protInfo-3party-t2.xml");
    if !env.protinfo.is_file() {
        eprintln!("skipping: no 3-of-2 protocol info file");
        return;
    }

    let dir = std::env::temp_dir().join("braid_vmnv_mix_inactive");
    emit_mixing::<3, 2>(&dir, &[1, 3]);

    let (code, output) = run_vmnv_mode(&env, &dir, "-mix", true);
    eprintln!("{output}");
    assert_eq!(code, 0, "vmnv -mix must accept a proof with a party that did not decrypt");
    assert_mix_verified(&output, 2);

    // The negative control for the claim above. Replace the excluded party's
    // identity commitment with a well-formed one over the generator: it is
    // still a valid group element, and it is still not combined into anything,
    // so the only way this can matter is through the challenge -- which is
    // exactly the property being asserted.
    //
    // The substitute has to *parse*. `setCommitment` falls back to the identity
    // on a malformed file, so corrupting bytes would leave the challenge
    // unchanged and prove nothing.
    use cryptography::traits::groups::GroupElement;
    let generator_commitment = vcompat::bytetree::ByteTree::node(vec![
        braid::vmn::encode::element_to_tree(&P256Element::generator()).unwrap(),
        braid::vmn::encode::elements_to_tree(&[P256Element::one(); W]).unwrap(),
    ]);
    std::fs::write(
        dir.join("proofs/DecrFactCommitment02.bt"),
        generator_commitment.to_bytes(),
    )
    .expect("overwrite the excluded party's commitment");

    let (code, output) = run_vmnv_mode(&env, &dir, "-mix", true);
    assert_ne!(
        code, 0,
        "an excluded party's commitment still enters the challenge, so changing \
         it must break the proof; got:\n{output}"
    );
    assert!(
        output.contains("Verify combined proof of decryption")
            && !output.contains("Verify combined proof of decryption... done."),
        "and it must break at the decryption proof specifically; got:\n{output}"
    );
}
