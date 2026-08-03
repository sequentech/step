// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `vsvmn` — interoperability between braid and Verificatum, from the command
//! line.
//!
//! Two directions, one each way across the boundary:
//!
//! - `generate` writes a session in Verificatum's format using our
//!   cryptography, for `vmnv` to check. It runs no external tool; the command to
//!   run is printed for the operator.
//! - `verify` checks a session Verificatum produced, using ours. It needs no
//!   JVM and reads nothing but the directory it is given.
//!
//! # Exit status
//!
//! `0` only when everything asked for was checked and passed. A rejected proof
//! and an unreadable one are both non-zero, and are reported differently: this
//! tool must never let "could not check" be read as "checked and passed", which
//! is the failure mode documented in [`vsvmn::verify`].

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{anyhow, bail, Context as _, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};

use vsvmn::emit::{self, SessionSpec};
use vsvmn::session::{self, ProofType};
use vsvmn::wire::protinfo::ProtocolInfo;

#[derive(Parser)]
#[command(
    name = "vsvmn",
    about = "Interoperate with Verificatum: generate sessions it can verify, verify sessions it produced",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write a session in Verificatum's format, using braid's cryptography.
    Generate(Generate),
    /// Verify a session Verificatum produced.
    Verify(Verify),
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum KindArg {
    /// A chain of mixers and nothing else; `vmnv -shuffle`.
    Shuffling,
    /// The whole session, ending in threshold decryption; `vmnv -mix`.
    Mixing,
}

#[derive(Args)]
struct Generate {
    /// Where to write. Receives `protInfo.xml` and a `nizkp/` proof directory.
    dir: PathBuf,

    /// Whether to include the decryption phase.
    #[arg(long, value_enum, default_value = "mixing")]
    kind: KindArg,

    /// Number of parties, k.
    #[arg(short = 'k', long, default_value_t = 3)]
    parties: usize,

    /// Parties needed to decrypt, lambda.
    #[arg(short = 't', long, default_value_t = 2)]
    threshold: usize,

    /// Ciphertext width, omega.
    #[arg(short = 'w', long, default_value_t = 2)]
    width: usize,

    /// How many ciphertexts to shuffle.
    #[arg(short = 'n', long, default_value_t = 100)]
    ciphertexts: usize,

    /// Which parties take part, 1-based and comma-separated; must name exactly
    /// lambda of them. Defaults to the first lambda. Parties left out get
    /// Verificatum's placeholder decryption material, so `--active 1,3` for
    /// k=3 is the case worth testing.
    #[arg(long, value_delimiter = ',')]
    active: Option<Vec<usize>>,

    /// Session identifier.
    #[arg(long, default_value = "braid")]
    sid: String,

    /// Auxiliary session identifier.
    #[arg(long, default_value = "default")]
    auxsid: String,

    /// Delete the output directory first if it already exists.
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
struct Verify {
    /// The protocol info file describing the session.
    protinfo: PathBuf,
    /// The proof directory, holding `type`, `width` and `proofs/`.
    dir: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Generate(args) => generate(&args),
        Command::Verify(args) => verify(&args),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        // A well-formed proof that does not verify. Distinguished from the
        // error case below because it is a *verdict*, not a failure to reach
        // one.
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn generate(args: &Generate) -> Result<bool> {
    let kind = match args.kind {
        KindArg::Shuffling => emit::Kind::Shuffling,
        KindArg::Mixing => emit::Kind::Mixing,
    };

    let mut spec = SessionSpec::p256(args.parties, args.threshold, args.width, args.ciphertexts);
    spec.info.sid = args.sid.clone();
    spec.auxsid = args.auxsid.clone();
    if let Some(active) = &args.active {
        spec.active = active.clone();
    }

    prepare(&args.dir, args.force)?;
    let nizkp = args.dir.join("nizkp");
    let protinfo = args.dir.join("protInfo.xml");

    // The info file is written from the same specification the proofs are
    // built from, so the parameters and the rho the verifier recomputes cannot
    // disagree. Handing over a file describing a different session is the one
    // way to make a correct proof look wrong.
    std::fs::write(&protinfo, spec.info.to_xml())
        .with_context(|| format!("writing {}", protinfo.display()))?;
    emit::generate(&spec, kind, &nizkp)?;

    let mode = match kind {
        emit::Kind::Shuffling => "-shuffle",
        emit::Kind::Mixing => "-mix",
    };
    println!(
        "wrote a {} session: k={}, lambda={}, omega={}, N={}, active={:?}",
        match kind {
            emit::Kind::Shuffling => "shuffling",
            emit::Kind::Mixing => "mixing",
        },
        spec.info.parties,
        spec.info.threshold,
        spec.info.width,
        spec.ciphertexts,
        spec.active
    );
    println!("  {}", protinfo.display());
    println!("  {}", nizkp.display());
    println!();
    println!("Verify it with Verificatum:");
    println!(
        "  vmnv {mode} -auxsid {} -width {} {} {}",
        spec.auxsid,
        spec.info.width,
        protinfo.display(),
        nizkp.display()
    );
    if cfg!(windows) {
        // `vmnv` itself runs anywhere -- it is Java verification. Its launcher
        // is a /bin/sh script, and running that under WSL would leave it with
        // Windows paths it cannot resolve, so the line above is not directly
        // runnable here.
        println!("  (on Windows the shipped `vmnv` is a shell script; see this");
        println!("   crate's README.md for invoking the verifier without WSL)");
    }
    println!("or with this tool:");
    println!("  vsvmn verify {} {}", protinfo.display(), nizkp.display());
    if kind == emit::Kind::Shuffling {
        // Worth saying where it is printed rather than in a manual: vmnv's exit
        // status is not a sound accept/reject signal for a shuffling proof.
        println!();
        println!("Note: `vmnv -shuffle` exits 0 even on proofs it rejected. Read its");
        println!("output for `Verify proof of shuffle... done.`, once per mixer.");
    }
    Ok(true)
}

fn verify(args: &Verify) -> Result<bool> {
    let xml = std::fs::read_to_string(&args.protinfo)
        .with_context(|| format!("reading {}", args.protinfo.display()))?;
    let info = ProtocolInfo::parse(&xml)
        .map_err(|e| anyhow!("{e:?}"))
        .with_context(|| format!("parsing {}", args.protinfo.display()))?;
    if !info.is_consistent() {
        bail!(
            "{} describes k={}, lambda={}, width={}, which is not a possible session",
            args.protinfo.display(),
            info.parties,
            info.threshold,
            info.width
        );
    }

    let meta = session::read_metadata(&args.dir)
        .with_context(|| format!("reading the proof directory {}", args.dir.display()))?;

    println!(
        "session: {}, k={}, lambda={}, omega={}, auxsid={}, active threshold={}",
        match meta.proof_type {
            ProofType::Mixing => "mixing",
            ProofType::Shuffling => "shuffling",
        },
        info.parties,
        info.threshold,
        meta.width,
        meta.auxsid,
        meta.active_threshold
    );

    // Width is a const generic, so dispatch on what the *proof* declares rather
    // than on what the info file says: a disagreement then fails here instead
    // of being silently assumed away.
    match meta.width {
        1 => report::<1>(&args.dir, &info, &meta),
        2 => report::<2>(&args.dir, &info, &meta),
        3 => report::<3>(&args.dir, &info, &meta),
        w => Err(anyhow!(
            "no verifier instantiation for width {w}; add one to main::verify"
        )),
    }
}

fn report<const W: usize>(
    dir: &Path,
    info: &ProtocolInfo,
    meta: &session::ProofMetadata,
) -> Result<bool> {
    let Some(outcome) = session::verify_session::<W>(dir, info, meta)? else {
        println!("REJECTED: a proof in this session does not verify");
        return Ok(false);
    };

    println!("  {} mixers verified", outcome.mixers_verified);
    println!("  {} ciphertexts in the output", outcome.shuffled.len());
    match &outcome.plaintexts {
        Some(p) => println!("  {} plaintexts recovered", p.len()),
        None => println!("  no decryption phase"),
    }

    // A chain shorter than the threshold is well formed and every proof in it
    // verifies; it is still not the session the info file describes. vmnv
    // accepts this in silence, so saying so is the whole point.
    if outcome.mixers_verified < info.threshold {
        println!(
            "REJECTED: only {} of the {} mixers this session requires left a proof",
            outcome.mixers_verified, info.threshold
        );
        return Ok(false);
    }

    println!("ACCEPTED");
    Ok(true)
}

/// Refuse to write over an existing directory unless told to.
fn prepare(dir: &Path, force: bool) -> Result<()> {
    if dir.exists() {
        if !force {
            bail!(
                "{} already exists; pass --force to replace it",
                dir.display()
            );
        }
        std::fs::remove_dir_all(dir).with_context(|| format!("replacing {}", dir.display()))?;
    }
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    Ok(())
}
