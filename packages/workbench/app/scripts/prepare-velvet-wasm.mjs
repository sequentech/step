// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Builds `workbench/velvet-wasm/pkg` and syncs it into
// `packages/node_modules/velvet-wasm`. Chained into `predev`/`prebuild`.
//
// The sync is the point. Yarn 1 resolves `file:` dependencies by
// *copying*, not symlinking, so `node_modules/velvet-wasm` is a snapshot
// taken at install time. Building `pkg/` alone leaves the app importing
// the stale copy: a changed export throws, but a change that only alters
// behaviour — a velvet-core tally fix, a sequent-core or strand bump
// underneath it — produces no error at all and the workbench reports old
// numbers with total confidence.

import {spawnSync} from "node:child_process"
import {fileURLToPath} from "node:url"
import fs from "node:fs"
import path from "node:path"

const here = path.dirname(fileURLToPath(import.meta.url))
const velvetWasm = path.resolve(here, "../../velvet-wasm")
const pkgDir = path.join(velvetWasm, "pkg")
const installed = path.resolve(here, "../../../node_modules/velvet-wasm")

console.log("[velvet-wasm] building workbench/velvet-wasm/pkg (wasm-pack)…")
const result = spawnSync(
    "wasm-pack",
    ["build", "--target", "web", "--out-dir", "pkg"],
    {cwd: velvetWasm, stdio: "inherit", shell: process.platform === "win32"}
)

if (result.error && result.error.code === "ENOENT") {
    console.error(
        "\n[velvet-wasm] wasm-pack not found on PATH.\n" +
            "[velvet-wasm] Install it: https://rustwasm.github.io/wasm-pack/"
    )
    process.exit(1)
}
if (result.status !== 0) process.exit(result.status ?? 1)

// Refresh the yarn-classic copy the app actually imports.
if (!fs.existsSync(installed)) {
    console.log(
        `[velvet-wasm] ${path.relative(process.cwd(), installed)} not present — skipping sync.\n` +
            "[velvet-wasm] Run `yarn install` from packages/ to create it."
    )
    process.exit(0)
}

let copied = 0
for (const entry of fs.readdirSync(pkgDir, {withFileTypes: true})) {
    if (!entry.isFile()) continue
    fs.copyFileSync(path.join(pkgDir, entry.name), path.join(installed, entry.name))
    copied++
}
console.log(`[velvet-wasm] synced ${copied} files into node_modules/velvet-wasm`)
