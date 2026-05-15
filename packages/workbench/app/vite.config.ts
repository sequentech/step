// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {defineConfig} from "vite"
import type {Plugin} from "vite"
import react from "@vitejs/plugin-react"
import wasm from "vite-plugin-wasm"
import topLevelAwait from "vite-plugin-top-level-await"
import {fileURLToPath} from "node:url"
import path from "node:path"
import fs from "node:fs"

const here = path.dirname(fileURLToPath(import.meta.url))
const pkgs = path.resolve(here, "../..")

// Build-time validator for the bundled snapshots under
// `src/fixtures/snapshots/`. Runs on every `vite dev` start and every
// `vite build` (both invoke `buildStart`). A bundled snapshot is the
// unit of state in the workbench, so any inconsistency here would
// surface as a confusing browser-side failure mode \u2014 we fail the
// build loudly instead.
//
// Invariants checked:
//   - `version === "v1"`
//   - Every entry in `state.ballotStyles` has a matching
//     `workbench.keypairs[ballotStyle.id]` with `pkB64` / `skB64`
//     strings. (The encrypt path uses the ballot style's
//     `public_key`; the decrypt bridge looks up the matching secret
//     key by ballot style id. A missing entry would silently break
//     the round-trip.)
//   - `state.ballotStyles[*].ballot_eml.public_key.public_key`\n//     matches `workbench.keypairs[ballotStyleId].pkB64`. Catches\n//     hand-edits that re-key one side and not the other.
//
// Implementation lives here rather than in `src/` because vite.config
// runs in node and cannot import browser-targeted TS modules.
function validateBundledSnapshots(): Plugin {
    const snapshotsDir = path.join(here, "src/fixtures/snapshots")
    return {
        name: "workbench-validate-bundled-snapshots",
        buildStart() {
            if (!fs.existsSync(snapshotsDir)) return
            const files = fs
                .readdirSync(snapshotsDir)
                .filter((f) => f.endsWith(".json"))
            for (const file of files) {
                const filePath = path.join(snapshotsDir, file)
                let snapshot: unknown
                try {
                    snapshot = JSON.parse(fs.readFileSync(filePath, "utf8"))
                } catch (e) {
                    this.error(
                        `Bundled snapshot ${file} is not valid JSON: ${
                            (e as Error).message
                        }`
                    )
                    return
                }
                const errors = validateSnapshot(snapshot)
                if (errors.length > 0) {
                    this.error(
                        `Bundled snapshot ${file} failed validation:\n  - ${errors.join(
                            "\n  - "
                        )}`
                    )
                }
            }
        },
    }
}

interface SnapshotShape {
    version?: unknown
    state?: {
        ballotStyles?: Record<
            string,
            {
                id?: unknown
                ballot_eml?: {public_key?: {public_key?: unknown}}
            }
        >
    }
    workbench?: {
        keypairs?: Record<string, {pkB64?: unknown; skB64?: unknown}>
    }
}

function validateSnapshot(raw: unknown): string[] {
    const errors: string[] = []
    if (!raw || typeof raw !== "object") {
        errors.push("snapshot is not an object")
        return errors
    }
    const s = raw as SnapshotShape
    if (s.version !== "v1") {
        errors.push(`expected version "v1", got ${JSON.stringify(s.version)}`)
    }
    const ballotStyles = s.state?.ballotStyles ?? {}
    const keypairs = s.workbench?.keypairs ?? {}
    for (const [key, bs] of Object.entries(ballotStyles)) {
        const bsId = bs?.id
        if (typeof bsId !== "string") {
            errors.push(
                `state.ballotStyles[${key}] has no string id`
            )
            continue
        }
        const kp = keypairs[bsId]
        if (!kp) {
            errors.push(
                `state.ballotStyles[${key}] (id=${bsId}) has no matching workbench.keypairs entry`
            )
            continue
        }
        if (typeof kp.pkB64 !== "string" || typeof kp.skB64 !== "string") {
            errors.push(
                `workbench.keypairs[${bsId}] is missing pkB64/skB64 strings`
            )
            continue
        }
        const pkInBallotStyle = bs?.ballot_eml?.public_key?.public_key
        if (
            typeof pkInBallotStyle === "string" &&
            pkInBallotStyle !== kp.pkB64
        ) {
            errors.push(
                `state.ballotStyles[${key}].ballot_eml.public_key.public_key does not match workbench.keypairs[${bsId}].pkB64`
            )
        }
    }
    return errors
}

// Vite config for the Sequentech workbench. `vite-plugin-wasm` lets us
// import the wasm-pack-generated `velvet-wasm` package as if it were any
// other ES module; `vite-plugin-top-level-await` is required because the
// wasm init is async.
//
// `resolve.alias` for `@sequentech/ui-core` and `@sequentech/ui-essentials`
// points Vite at the TypeScript sources of those workspace packages
// rather than at their (uncompiled) `dist/` outputs. Compiling them on
// the fly avoids running their webpack/tsc build step before each dev
// session, and keeps the workbench faithful to the same source the
// production voting-portal consumes — only the *bundler* differs.
export default defineConfig({
    plugins: [react(), wasm(), topLevelAwait(), validateBundledSnapshots()],
    resolve: {
        alias: [
            // Point package-name imports at the workspace TS sources so
            // Vite compiles them on the fly (no need to run their
            // webpack/tsc build before each dev session). Keeps the
            // workbench faithful to the same source the production
            // voting-portal consumes — only the *bundler* differs.
            {
                find: "@sequentech/ui-core",
                replacement: path.join(pkgs, "ui-core/src/index.tsx"),
            },
            {
                find: "@sequentech/ui-essentials",
                replacement: path.join(pkgs, "ui-essentials/src/index.tsx"),
            },
            // ui-core's own internal imports use a `@root/*` tsconfig
            // path alias resolving to `ui-core/src/*`. Vite doesn't read
            // tsconfig paths, so reproduce it here. (ui-essentials
            // doesn't use `@root` internally — verified by grep.)
            {
                find: /^@root\/(.*)$/,
                replacement: path.join(pkgs, "ui-core/src/$1"),
            },
        ],
    },
    server: {
        port: 5173,
        strictPort: true,
    },
    optimizeDeps: {
        // Both velvet-wasm (workbench's own) and sequent-core (the npm
        // package voting-portal consumes) ship an ES module that
        // computes its `.wasm` URL via `new URL("..._bg.wasm",
        // import.meta.url)`. Vite's dep optimizer rewrites
        // `import.meta.url` to a path under `.vite/deps/`, where the
        // wasm binary doesn't exist — the dev server then SPA-falls
        // back to `index.html` and the wasm loader fails with
        // "expected magic word 00 61 73 6d, found 3c 21 2d 2d" (the
        // bytes of "<!--"). Excluding these packages from
        // optimization keeps the original `import.meta.url`,
        // pointing at the real `.wasm` next to the JS.
        exclude: ["velvet-wasm", "sequent-core"],
    },
})
