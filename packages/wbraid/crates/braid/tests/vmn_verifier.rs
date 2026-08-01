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

#![cfg(feature = "native")]

use std::path::PathBuf;
use std::process::Command;

use braid::vmn::{challenges::VmnChallenges, generators::vmn_generators, proof_dir::ShufflingProof};
use cryptography::context::{Context, P256Ctx};
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
    let mut command = Command::new(&env.java);
    command
        .arg("-cp")
        .arg(&env.classpath)
        .arg(VERIFY_TOOL)
        // The launcher script passes these three ahead of the real arguments.
        .arg("vmnv")
        .arg(&env.random_source)
        .arg(&env.random_seed)
        .arg("-shuffle");
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

    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code().unwrap_or(-1), text)
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
    let challenges = VmnChallenges::new(Hashfunction::Sha256, rho, N_E, N_V, W);
    let (output, proof) = shuffler
        .shuffle_with(&input, &[], &challenges)
        .expect("shuffle");

    ShufflingProof::<W> {
        version: "3.1.0",
        auxsid: AUXSID,
        width: W,
        public_key: &keypair.pkey.y,
        input: &input,
        output: &output,
        proof: &proof,
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

    let (code, output) = run_vmnv(&env, &dir, true);
    eprintln!("{output}");
    assert_eq!(code, 0, "vmnv must accept a proof braid produced");
    assert!(
        output.contains("Verify proof of shuffle... done."),
        "vmnv must actually run the shuffle verification, not skip it"
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
