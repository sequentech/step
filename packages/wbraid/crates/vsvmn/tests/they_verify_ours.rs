// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Run Verificatum's `vmnv` against a proof braid produced.
//!
//! `we_verify_theirs.rs` covers the opposite direction — our verifier against a
//! session Verificatum generated. This is the other direction, and it is the
//! claim the whole exercise rests on: **an independently written verifier, in a
//! different language, accepts braid's output.** Without this test that claim
//! would only be reproducible by hand.
//!
//! `#[ignore]` because it shells out to a JVM, following the convention of the
//! HTTP protocol tests that need a live b4. Run with:
//!
//! ```text
//! cargo test -p vsvmn --test they_verify_ours -- --ignored --nocapture
//! ```
//!
//! Nothing needs setting up beyond Verificatum itself: the random source
//! `vmnv` insists on is provisioned by `common::random_source`. Verificatum is
//! looked for at `crates/braid/verificatum`, which is not part of this
//! repository — see TESTING.md for what goes in it. `VMN_HOME` points
//! elsewhere, `VMN_JAVA` at a different `java`, and
//! `VMN_RANDOM_SOURCE`/`VMN_RANDOM_SEED` at a source initialised by hand.
//!
//! The protocol info file is synthesized per session, so its parameters and the
//! global prefix rho the verifier recomputes cannot drift apart. `VMN_PROTINFO`
//! overrides it, which is only useful for checking a file from elsewhere.
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


use std::path::PathBuf;
use std::process::Command;

// Shared with the other interop tests: locating Verificatum, and the random
// source it refuses to start without.
#[allow(dead_code)]
mod common;

// The shipped emitter, so these tests exercise what the tool ships rather
// than a second copy of it.
use vsvmn::emit::{mixing, shuffling, shuffling_with_prefix, SessionSpec};
use vsvmn::wire::protinfo::ProtocolInfo;

const W: usize = 2;
const N: usize = 8;
const SID: &str = "braidpoc";
const AUXSID: &str = "default";


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
    let (random_source, random_seed) = common::random_source()?;

    Some(Env {
        java: common::java(),
        classpath: common::classpath()?,
        // Synthesized by default; every test that cares sets its own. Nothing
        // is read from disk, so no shape is privileged by being checked in.
        protinfo: match std::env::var("VMN_PROTINFO") {
            Ok(p) => PathBuf::from(p),
            Err(_) => write_protinfo(&spec(1, 1).info, "default"),
        },
        random_source: random_source.clone(),
        random_seed: random_seed.clone(),
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
/// arrays into a working directory under `/tmp/com.verificatum` and deletes it
/// on exit, so two processes sharing one kill each other with `File not found!`
/// or `Unable to delete storage directory!` part-way through a proof.
///
/// Without `-wd`, `TempFile.init` names that directory from
/// `randomSource.getBytes(10)` — which is only as unique as the random source
/// is. Ours is a **seeded PRG**, because Windows has no `/dev/urandom` (see the
/// corpus README), so it is deterministic: concurrent runs all read the same
/// seed file before any of them rewrites it, derive the same bytes, and collide.
/// On Unix with `vog -rndinit RandomDevice /dev/urandom` the source is genuinely
/// random and this would not arise, so it is an artifact of the Windows setup
/// rather than an upstream bug.
///
/// It must be a relative name: `TempFile` treats a path as absolute only if it
/// starts with `/`, so a Windows path would be appended to the default root
/// rather than replacing it.
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
/// — so this is the second file concurrent runs would share, and sharing mutable
/// state across processes to save a file copy is not a trade worth making.
///
/// Note this does **not** fix the working-directory collision `-wd` addresses:
/// the copies are byte-identical, so every run still derives the same "random"
/// bytes from them. The random *source* file is only read, and stays shared.
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

/// The session these fixed-shape tests describe.
///
/// One specification builds both the proofs and the protocol info file handed
/// to `vmnv`, so the parameters and the rho it recomputes cannot disagree --
/// which is the one way to make a correct proof look wrong.
fn spec(parties: usize, threshold: usize) -> SessionSpec {
    let mut spec = SessionSpec::p256(parties, threshold, W, N);
    spec.info.sid = SID.to_string();
    spec.auxsid = AUXSID.to_string();
    spec
}

/// A single-mixer shuffling proof, through the shipped emitter.
fn emit(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
    shuffling::<W>(&spec(1, 1), dir).expect("emit a shuffling proof");
}

/// As [`emit`], but `drift` perturbs the global prefix to stand in for a
/// regression in braid's transcript layer -- the realistic way this interop
/// breaks. The resulting proof is well-formed but does not verify.
///
/// The perturbation is applied to rho rather than by using a different emitter,
/// so what is under test is the shipped one.
fn emit_with_drift(dir: &PathBuf, drift: bool) {
    let _ = std::fs::remove_dir_all(dir);
    let spec = spec(1, 1);
    let mut rho = spec.prefix();
    if drift {
        rho[0] ^= 0x01;
    }
    shuffling_with_prefix::<W>(&spec, &rho, dir).expect("emit a shuffling proof");
}

/// A chain of `mixers` mixers, written as one multi-party shuffling proof.
fn emit_chain(dir: &PathBuf, mixers: usize) {
    emit_chain_with_threshold(dir, mixers, mixers);
}

/// As [`emit_chain`], with the session party count given separately. The
/// emitter runs one mixer per active party, and lambda_a >= lambda, so
/// `parties >= mixers`.
fn emit_chain_with_threshold(dir: &PathBuf, mixers: usize, parties: usize) {
    let _ = std::fs::remove_dir_all(dir);
    shuffling::<W>(&spec(parties, mixers), dir).expect("emit a shuffling chain");
}

/// Corrupting any single mixer's proof must sink the whole chain, so a chain is
/// not accepted on the strength of its other members.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_rejects_a_chain_with_one_bad_mixer() {
    let Some(mut env) = env() else {
        return common::skip("Verificatum cannot be run here");
    };
    env.protinfo = write_protinfo(&spec(3, 3).info, "3of3");

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
        return common::skip("Verificatum cannot be run here");
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
        return common::skip("Verificatum cannot be run here");
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
        return common::skip("Verificatum cannot be run here");
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
        return common::skip("Verificatum cannot be run here");
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

/// A complete mixing session at `(K, T)`, through the shipped emitter.
///
/// `delta` names the 1-based indices of the T parties that decrypt; any party
/// not listed contributes the all-identity factor array and the identity
/// commitment and zero reply Verificatum records for an absent contribution.
fn emit_mixing<const K: usize, const T: usize>(dir: &PathBuf, delta: &[usize]) {
    let _ = std::fs::remove_dir_all(dir);
    let mut spec = spec(K, T);
    spec.active = delta.to_vec();
    mixing::<W, K, T>(&spec, dir).expect("emit a mixing proof");
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

// -------------------------------------------------------------------------
// Synthesized protocol info files
// -------------------------------------------------------------------------


/// Write a synthesized protocol info file and return its path.
fn write_protinfo(info: &ProtocolInfo, name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("{}_{name}.xml", private_name("protinfo")));
    std::fs::write(&path, info.to_xml()).expect("write the synthesized protocol info");
    path
}

/// **`vmnv` accepts a protocol info file we generated.**
///
/// Until now every shape had to be a file checked into `testdata/`, generated
/// under WSL with `vmni` because the prover-side tools are Unix-bound. That
/// caps the testable shapes at the three we happened to generate.
///
/// A synthesized file gives every party the same signature key, which is sound
/// here only because Fiat–Shamir verification checks no signatures. This test is
/// what establishes that `vmnv` agrees — without it the synthesis is an
/// assumption.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_accepts_a_synthesized_protocol_info_file() {
    let Some(mut env) = env() else {
        return common::skip("Verificatum cannot be run here");
    };

    // The same 3-of-3 session as the shipped file, but written by us.
    env.protinfo = write_protinfo(&spec(3, 3).info, "3of3");

    let dir = std::env::temp_dir().join("braid_vmnv_synth");
    emit_chain(&dir, 3);

    let (code, output) = run_vmnv(&env, &dir, true);
    assert_eq!(code, 0, "vmnv must accept a synthesized info file:\n{output}");
    assert_eq!(
        output.matches("Verify proof of shuffle... done.").count(),
        3,
        "all three shuffles must verify against it; got:\n{output}"
    );
}

/// **The parameter sweep the synthesis exists for.**
///
/// Cross-implementation testing over a *range* of session shapes rather than the
/// three that happen to have checked-in info files. Each `(k, λ)` pair gets a
/// generated file and a chain of `λ` mixers, and unmodified `vmnv` must accept
/// every one.
///
/// This is the part that could not be done before: `vmnv` takes `k` and `λ` from
/// the info file, so testing a shape means having a file for that shape.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_accepts_a_sweep_of_session_shapes() {
    let Some(mut env) = env() else {
        return common::skip("Verificatum cannot be run here");
    };

    for parties in 1..=4 {
        for threshold in 1..=parties {
            let info = spec(parties, threshold).info;
            assert!(info.is_consistent());
            env.protinfo = write_protinfo(&info, &format!("{parties}of{threshold}"));

            // lambda_a >= lambda, and the emitter derives the active threshold
            // from the number of mixers.
            let dir = std::env::temp_dir()
                .join(format!("braid_vmnv_sweep_{parties}_{threshold}"));
            emit_chain(&dir, threshold);

            let (code, output) = run_vmnv(&env, &dir, true);
            assert_eq!(
                code, 0,
                "vmnv rejected a {parties}-party threshold-{threshold} session:\n{output}"
            );
            assert_eq!(
                output.matches("Verify proof of shuffle... done.").count(),
                threshold,
                "every mixer must verify at k={parties}, lambda={threshold}; got:\n{output}"
            );
            eprintln!("ok: k={parties}, lambda={threshold}");
            let _ = std::fs::remove_dir_all(&dir);
            let _ = std::fs::remove_file(&env.protinfo);
        }
    }
}

/// Runtime `(k, λ)` to the const-generic emitter.
///
/// `emit_mixing` is generic over both because the DKG's polynomial degree and
/// participant count are compile-time in vsc, so a sweep needs one arm per
/// shape. Unsupported combinations panic rather than being silently skipped.
fn emit_mixing_shape(dir: &PathBuf, parties: usize, threshold: usize, delta: &[usize]) {
    match (parties, threshold) {
        (1, 1) => emit_mixing::<1, 1>(dir, delta),
        (2, 2) => emit_mixing::<2, 2>(dir, delta),
        (3, 2) => emit_mixing::<3, 2>(dir, delta),
        (3, 3) => emit_mixing::<3, 3>(dir, delta),
        (4, 2) => emit_mixing::<4, 2>(dir, delta),
        (4, 3) => emit_mixing::<4, 3>(dir, delta),
        (4, 4) => emit_mixing::<4, 4>(dir, delta),
        (k, t) => panic!("no emitter arm for k={k}, lambda={t}; add one"),
    }
}

/// **The mixing counterpart of [`vmnv_accepts_a_sweep_of_session_shapes`].**
///
/// That sweep covers shuffling, where the emitted proof carries no decryption at
/// all. This one covers the whole session — DKG, chain, threshold decryption —
/// over a range of `(k, λ)`, so α, the modified Lagrange coefficients and the
/// inactive-party placeholders are exercised at more than the two shapes the
/// fixed tests pin.
///
/// Where `λ < k` it runs twice: once with Δ as the leading parties, and once
/// with Δ spread to the ends so the inactive parties fall *between* active ones.
/// The second is the arrangement that would expose an off-by-one in party
/// indexing, and it is also the shape where VMN leaves a gap in the mixer slots.
#[test]
#[ignore = "requires a JVM and the Verificatum jars; see the module docs"]
fn vmnv_accepts_a_sweep_of_mixing_shapes() {
    let Some(mut env) = env() else {
        return common::skip("Verificatum cannot be run here");
    };

    for (parties, threshold) in [(1, 1), (2, 2), (3, 2), (3, 3), (4, 2), (4, 3), (4, 4)] {
        // Leading parties, and -- when some sit out -- a spread set.
        let leading: Vec<usize> = (1..=threshold).collect();
        let spread: Vec<usize> = if threshold < parties {
            let mut d: Vec<usize> = (1..threshold).collect();
            d.push(parties);
            d
        } else {
            leading.clone()
        };

        let mut deltas = vec![leading];
        if deltas[0] != spread {
            deltas.push(spread);
        }

        for delta in deltas {
            let info = spec(parties, threshold).info;
            env.protinfo = write_protinfo(&info, &format!("mix{parties}of{threshold}"));

            let tag: String = delta.iter().map(usize::to_string).collect();
            let dir = std::env::temp_dir()
                .join(format!("braid_vmnv_mixsweep_{parties}_{threshold}_{tag}"));
            emit_mixing_shape(&dir, parties, threshold, &delta);

            let (code, output) = run_vmnv_mode(&env, &dir, "-mix", true);
            assert_eq!(
                code, 0,
                "vmnv rejected k={parties}, lambda={threshold}, delta={delta:?}:\n{output}"
            );
            assert_mix_verified(&output, threshold);
            eprintln!("ok: k={parties}, lambda={threshold}, delta={delta:?}");

            let _ = std::fs::remove_dir_all(&dir);
            let _ = std::fs::remove_file(&env.protinfo);
        }
    }
}
