// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Finite engine execution shared by the CLI and the standalone container worker.
use super::{files, input::Input, Engine};
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    process::{Command, Stdio},
};

/// Infrastructure variables needed by engine runtimes; administrator secrets are excluded.
const ENGINE_ENV: &[&str] = &[
    "PATH",
    "HOME",
    "TMPDIR",
    "LD_LIBRARY_PATH",
    "NIX_LD",
    "NIX_LD_LIBRARY_PATH",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "NODE_EXTRA_CA_CERTS",
    "FONTCONFIG_FILE",
    "FONTCONFIG_PATH",
    "PLAYWRIGHT_BROWSERS_PATH",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "no_proxy",
];

/// Start an engine with only infrastructure configuration and the synthetic password.
pub fn environment(command: &mut Command, input: &Input, config: &Path) -> Result<()> {
    command.env_clear();
    for name in ENGINE_ENV {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .env("LOAD_CONFIG", config)
        .env("LOAD_PASSWORD", input.password()?);
    Ok(())
}

/// Resolve the installed Playwright entry point using the configured Node runtime.
pub fn playwright(node: &str, directory: &Path) -> Result<std::path::PathBuf> {
    let output = Command::new(node)
        .args(["-p", "require.resolve('@playwright/test/package.json')"])
        .current_dir(directory)
        .output()?;
    ensure!(
        output.status.success(),
        "Cannot resolve Playwright in {}",
        directory.display()
    );
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

/// Attempt one shard once; even a launch failure leaves its durable claim behind.
pub fn shard(directory: &Path, shard: usize, assets: &Path) -> Result<()> {
    let config_path = directory.join("config.json");
    let input: Input = files::read(&config_path)?;
    input.validate()?;
    input.bounds(shard)?;
    let ready: Value = files::read(&directory.join("ready.json"))?;
    ensure!(
        ready["config_sha256"].as_str() == Some(&files::digest(&config_path)?),
        "Prepared configuration changed; prepare a fresh run"
    );
    let ballot = directory.join(format!("{shard:06}.jsonl"));
    if matches!(input.settings.workload.engine, Engine::K6)
        && input.settings.workload.mode == "vote"
    {
        ensure!(
            files::digest(&ballot)? == fs::read_to_string(ballot.with_extension("sha256"))?,
            "Prepared ciphertext shard changed"
        );
    }
    let output = directory.join("results").join(format!("{shard:06}"));
    files::directory(&output)?;
    files::create(&output.join("attempted"))?.sync_all()?;
    let mut runtime = input.settings.runtime.clone();
    if std::env::var_os("STEP_LOAD_CONTAINER").is_some() {
        runtime.node = "node".into();
        runtime.k6 = "k6".into();
        runtime.playwright_dir = "/runner".into();
        runtime.chromium = None;
    }
    let mut node_modules = runtime.playwright_dir.join("node_modules");
    let mut command = match input.settings.workload.engine {
        Engine::K6 => {
            let mut command = Command::new(&runtime.k6);
            command
                .args(["run", "--log-format", "raw"])
                .arg(assets.join("packages/voting-load/scale.k6.js"));
            command
        }
        Engine::Chromium => {
            let package = playwright(&runtime.node, &runtime.playwright_dir)?;
            node_modules = package
                .ancestors()
                .nth(3)
                .context("Playwright node_modules directory missing")?
                .to_path_buf();
            let mut command = Command::new(&runtime.node);
            command
                .arg(
                    package
                        .parent()
                        .context("Playwright package directory missing")?
                        .join("cli.js"),
                )
                .arg("test")
                .arg("--config")
                .arg(assets.join("packages/voting-portal/playwright.scale.config.ts"));
            command
        }
    };
    environment(&mut command, &input, &config_path)?;
    command
        .env("LOAD_SHARD", shard.to_string())
        .env("LOAD_BALLOTS", &ballot)
        .env("LOAD_SUMMARY", output.join("summary.json"))
        .env("LOAD_RESULTS", output.join("samples.jsonl"))
        .env("LOAD_ARTIFACTS", output.join("browser"))
        .env(
            "LOAD_ACTION_TIMEOUT_MS",
            input.settings.workload.action_timeout_ms.to_string(),
        )
        .env("NODE_PATH", node_modules);
    if let Some(chromium) = runtime.chromium {
        command.env("CHROMIUM_EXECUTABLE_PATH", chromium);
    }
    let log = files::create(&output.join("worker.log"))?;
    let result = command
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .status();
    if matches!(input.settings.workload.engine, Engine::K6) {
        let mut samples = BufWriter::new(files::create(&output.join("samples.jsonl"))?);
        for line in BufReader::new(fs::File::open(output.join("worker.log"))?).lines() {
            if let Some(sample) = line?.strip_prefix("RESULT ") {
                writeln!(samples, "{sample}")?;
            }
        }
        samples.flush()?;
    }
    let code = result
        .as_ref()
        .ok()
        .and_then(|status| status.code())
        .unwrap_or(-1);
    files::save(&output.join("exit.json"), &json!({"code":code}))?;
    ensure!(
        result.is_ok_and(|status| status.success()),
        "Shard {shard} failed; inspect {}",
        output.join("worker.log").display()
    );
    Ok(())
}

/// An indexed worker owns disjoint shards and never holds multiple shards in memory.
pub fn node(directory: &Path, index: usize, workers: usize, assets: &Path) -> Result<()> {
    ensure!(
        workers > 0 && index < workers,
        "Worker index must be below positive worker count"
    );
    let input: Input = files::read(&directory.join("config.json"))?;
    input.validate()?;
    let mut failure = None;
    for next in (index..input.shards()).step_by(workers) {
        if let Err(error) = shard(directory, next, assets) {
            failure.get_or_insert(error);
        }
    }
    if let Some(error) = failure {
        bail!("Worker {index} did not complete all its shards: {error:#}");
    }
    Ok(())
}
