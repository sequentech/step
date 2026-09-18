// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Build workers from an explicit source-only context; no run artifacts reach Docker.
use super::Engine;
use anyhow::{ensure, Context, Result};
use std::{
    path::Path,
    process::{Command, Stdio},
};

/// Build and optionally publish a worker with operator-selectable engine/base versions.
#[allow(clippy::too_many_arguments)]
pub fn build(
    root: &Path,
    engine: Engine,
    tag: &str,
    push: bool,
    rust: &str,
    k6: &str,
    worker: &str,
    browser: &str,
    playwright: &str,
) -> Result<()> {
    let paths = [
        "packages/voting-load/Dockerfile",
        "packages/voting-load/worker.Cargo.toml",
        "packages/voting-load/worker.Cargo.lock",
        "packages/voting-load/worker.rs",
        "packages/voting-load/scale.k6.js",
        "packages/voting-load/replay.k6.js",
        "packages/voting-load/bootstrap.k6.js",
        "packages/step-cli/src/load/config.rs",
        "packages/step-cli/src/load/files.rs",
        "packages/step-cli/src/load/input.rs",
        "packages/step-cli/src/load/worker.rs",
        "packages/voting-portal/src/queries/GetVoterStatus.ts",
        "packages/voting-portal/src/queries/InsertCastVote.ts",
        "packages/voting-portal/playwright.scale.config.ts",
        "packages/voting-portal/test/load/flow.ts",
        "packages/voting-portal/test/load/scale.spec.ts",
    ];
    let mut tar = Command::new("tar")
        .arg("-C")
        .arg(root)
        .args(["-cf", "-"])
        .args(paths)
        .stdout(Stdio::piped())
        .spawn()?;
    let mut docker = Command::new("docker");
    if Command::new("docker")
        .args(["buildx", "version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success()
    {
        docker.args(["buildx", "build", "--load"]);
    } else {
        docker.arg("build");
    }
    docker.args([
        "--target",
        if matches!(engine, Engine::K6) {
            "k6"
        } else {
            "chromium"
        },
        "--tag",
        tag,
        "--file",
        "packages/voting-load/Dockerfile",
    ]);
    for (name, value) in [
        ("RUST_IMAGE", rust),
        ("K6_IMAGE", k6),
        ("WORKER_IMAGE", worker),
        ("BROWSER_IMAGE", browser),
        ("PLAYWRIGHT_VERSION", playwright),
    ] {
        docker.arg("--build-arg").arg(format!("{name}={value}"));
    }
    let build = docker
        .arg("-")
        .stdin(Stdio::from(
            tar.stdout.take().context("Source archive unavailable")?,
        ))
        .status();
    let archive = tar.wait()?;
    ensure!(
        build?.success() && archive.success(),
        "Worker image build failed"
    );
    if push {
        ensure!(
            Command::new("docker")
                .args(["push", tag])
                .status()?
                .success(),
            "Worker image push failed"
        );
    }
    Ok(())
}
