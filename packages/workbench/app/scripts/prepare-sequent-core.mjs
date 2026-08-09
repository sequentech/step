// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Builds `packages/sequent-core/pkg` so the lifted booth runs the same
// sequent-core source that velvet-core/velvet-wasm compile into the tally
// half. Chained into `predev` / `prebuild`.
//
// The workbench deliberately does NOT use the committed
// `rust/sequent-core-0.1.0.tgz` by default. That artifact is a snapshot
// packed from this same `pkg/` at whatever commit someone last ran
// `.devcontainer/scripts/build-sequent-core.sh`, and defaulting to it
// caused two recurring problems: local Rust edits silently not showing
// up, and the booth (tgz) disagreeing with the tally (source) with no
// error — just wrong numbers.
//
// Opt back into the tarball explicitly:
//
//     WORKBENCH_SEQUENT_CORE=tgz yarn dev
//
// which skips this build and stops `vite.config.ts` registering the
// alias. Use it when you need to reproduce what a deployed booth does,
// since the tgz is the artifact production ships.

import {spawnSync} from "node:child_process"
import {fileURLToPath} from "node:url"
import path from "node:path"

const here = path.dirname(fileURLToPath(import.meta.url))
const sequentCore = path.resolve(here, "../../../sequent-core")

const source = process.env.WORKBENCH_SEQUENT_CORE ?? "local"

if (source === "tgz") {
    console.log(
        "[sequent-core] WORKBENCH_SEQUENT_CORE=tgz — skipping the local wasm-pack build.\n" +
            "[sequent-core] The booth will use the committed rust/sequent-core-0.1.0.tgz.\n" +
            "[sequent-core] Note this can disagree with the tally half, which always\n" +
            "[sequent-core] compiles sequent-core from source via velvet-wasm."
    )
    process.exit(0)
}

if (source !== "local") {
    console.error(
        `[sequent-core] Unknown WORKBENCH_SEQUENT_CORE=${source}. Expected "local" (default) or "tgz".`
    )
    process.exit(1)
}

// Same feature set `.devcontainer/scripts/build-sequent-core.sh` uses to
// pack the committed tarball, so switching between the two sources with
// WORKBENCH_SEQUENT_CORE compares like with like rather than against a
// build production never produced.
const args = [
    "build",
    "--out-name",
    "index",
    "--release",
    "--target",
    "web",
    "--features=wasmtest,default_features",
]

console.log("[sequent-core] building packages/sequent-core/pkg (wasm-pack)…")
const result = spawnSync("wasm-pack", args, {
    cwd: sequentCore,
    stdio: "inherit",
    shell: process.platform === "win32",
})

if (result.error && result.error.code === "ENOENT") {
    console.error(
        "\n[sequent-core] wasm-pack not found on PATH.\n" +
            "[sequent-core] Install it (https://rustwasm.github.io/wasm-pack/), or run with\n" +
            "[sequent-core]   WORKBENCH_SEQUENT_CORE=tgz\n" +
            "[sequent-core] to use the committed tarball instead."
    )
    process.exit(1)
}

process.exit(result.status ?? 1)
