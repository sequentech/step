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
//   - `workbench.keypair` is present and has string `pkB64` / `skB64`.
//   - Every `state.ballotStyles[*].ballot_eml.public_key.public_key`
//     equals `workbench.keypair.pkB64`. Catches hand-edits that
//     re-key one side and not the other, and snapshots whose ballot
//     styles drifted apart on the public-key field (which would break
//     the encrypt/decrypt round-trip silently).
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

// Non-intrusive build-info reporter. Exposes a virtual ES module
// `virtual:workbench-build-info` whose default export is a small
// snapshot of when each wasm artifact under the workspace was last
// (re)built, together with the newest source mtime under its crate
// (transitively, for the path-deps it depends on). The homepage
// renders this so a contributor can spot at a glance whether the
// wasm they're seeing in the browser is older than the Rust source
// — the common footgun behind "I edited Rust, why didn't it
// change". No Rust changes, no extra yarn scripts: the plugin
// simply `fs.stat`s files Vite already knows about.
//
// Watcher invalidation: each artifact + crate src/ tree is added to
// Vite's chokidar watcher; any change invalidates this virtual
// module so a browser refresh re-reads the mtimes.
interface BuildArtifactConfig {
    label: string
    artifact: string // absolute path to *_bg.wasm
    /**
     * Top-level cargo crate name (matches `[package].name` in the
     * crate's Cargo.toml and `[[package]].name` in Cargo.lock) whose
     * dep tree contributes symbols to this wasm. Used as the root for
     * the Cargo.lock walk that surfaces transitive resolved versions.
     */
    cargoCrate: string
}

function workbenchBuildInfo(): Plugin {
    const VIRTUAL_ID = "virtual:workbench-build-info"
    const RESOLVED_ID = "\0" + VIRTUAL_ID

    const cargoLockPath = path.resolve(pkgs, "Cargo.lock")
    const repoRoot = path.resolve(pkgs, "..")

    const artifacts: BuildArtifactConfig[] = [
        {
            label: "sequent-core (wasm)",
            artifact: path.resolve(pkgs, "sequent-core/pkg/index_bg.wasm"),
            cargoCrate: "sequent-core",
        },
        {
            label: "velvet-wasm",
            artifact: path.resolve(
                pkgs,
                "workbench/velvet-wasm/pkg/velvet_wasm_bg.wasm"
            ),
            cargoCrate: "velvet-wasm",
        },
    ]

    // -----------------------------------------------------------------
    // Minimal Cargo.lock walker. We avoid adding a TOML dep at
    // config-time and instead parse just the bits we need: each
    // `[[package]]` section's `name`, `version`, optional `source =`,
    // and the `dependencies = [...]` array. That is enough to BFS
    // the resolved dep graph from a root crate and tell which
    // workspace-internal (path-dep, i.e. no `source = `) crates were
    // linked in, with what version. Transitive registry crates are
    // counted but not enumerated, so the UI stays readable.
    // -----------------------------------------------------------------
    interface LockPackage {
        name: string
        version: string
        source: string | null // null => workspace-internal path dep
        deps: string[] // raw "name [version [(source)]]" entries
    }

    function parseCargoLock(text: string): LockPackage[] {
        const out: LockPackage[] = []
        const lines = text.split(/\r?\n/)
        let i = 0
        while (i < lines.length) {
            if (lines[i].trim() === "[[package]]") {
                i++
                let name = ""
                let version = ""
                let source: string | null = null
                const deps: string[] = []
                while (
                    i < lines.length &&
                    lines[i].trim() !== "[[package]]" &&
                    !lines[i].startsWith("[")
                ) {
                    const line = lines[i]
                    const m = /^([a-zA-Z_]+)\s*=\s*(.*)$/.exec(line.trim())
                    if (m) {
                        const key = m[1]
                        const rest = m[2]
                        if (key === "name") name = stripQuotes(rest)
                        else if (key === "version") version = stripQuotes(rest)
                        else if (key === "source") source = stripQuotes(rest)
                        else if (key === "dependencies" && rest.startsWith("[")) {
                            // multi-line array: collect entries until
                            // the closing bracket.
                            i++
                            while (i < lines.length && !lines[i].includes("]")) {
                                const entry = lines[i]
                                    .trim()
                                    .replace(/,$/, "")
                                if (entry.startsWith('"') && entry.endsWith('"')) {
                                    deps.push(stripQuotes(entry))
                                }
                                i++
                            }
                        }
                    }
                    i++
                }
                if (name && version) {
                    out.push({name, version, source, deps})
                }
                continue
            }
            i++
        }
        return out
    }

    function stripQuotes(v: string): string {
        const t = v.trim()
        if (t.startsWith('"') && t.endsWith('"')) return t.slice(1, -1)
        return t
    }

    interface ResolvedDep {
        name: string
        version: string
        internal: boolean // true iff no `source =` in the lock entry
    }

    function walkLock(
        rootCrate: string,
        packages: LockPackage[]
    ): ResolvedDep[] | null {
        // Index by name -> entries (multiple versions possible).
        const byName = new Map<string, LockPackage[]>()
        for (const p of packages) {
            const list = byName.get(p.name) ?? []
            list.push(p)
            byName.set(p.name, list)
        }
        const roots = byName.get(rootCrate)
        if (!roots || roots.length === 0) return null
        // If multiple versions exist for the root, pick the path-dep
        // one (no source) — that's the in-tree workspace member.
        const root = roots.find((p) => p.source === null) ?? roots[0]
        const seen = new Set<string>()
        const key = (p: LockPackage): string => `${p.name} ${p.version}`
        const queue: LockPackage[] = [root]
        const result: LockPackage[] = []
        while (queue.length > 0) {
            const cur = queue.shift()!
            const k = key(cur)
            if (seen.has(k)) continue
            seen.add(k)
            result.push(cur)
            for (const dep of cur.deps) {
                const resolved = resolveDep(dep, byName)
                if (resolved) queue.push(resolved)
            }
        }
        return result.map((p) => ({
            name: p.name,
            version: p.version,
            internal: p.source === null,
        }))
    }

    function resolveDep(
        entry: string,
        byName: Map<string, LockPackage[]>
    ): LockPackage | null {
        // Entry shapes (cargo writes the shortest unambiguous form):
        //   "name"
        //   "name version"
        //   "name version (source)"
        const m = /^([^\s]+)(?:\s+([^\s]+)(?:\s+\((.*)\))?)?$/.exec(entry)
        if (!m) return null
        const name = m[1]
        const version = m[2]
        const source = m[3] // may be undefined
        const candidates = byName.get(name)
        if (!candidates) return null
        if (candidates.length === 1) return candidates[0]
        // Disambiguate by version (and source if present).
        for (const c of candidates) {
            if (version && c.version !== version) continue
            if (source !== undefined && c.source !== source) continue
            return c
        }
        return null
    }

    // Read Cargo.lock once per snapshot; the file is ~300KB and the
    // walks are cheap, but caching the parse for the duration of one
    // snapshot() call avoids re-parsing per artifact.
    function readLockPackages(): LockPackage[] | null {
        if (!fs.existsSync(cargoLockPath)) return null
        try {
            return parseCargoLock(fs.readFileSync(cargoLockPath, "utf8"))
        } catch {
            return null
        }
    }

    function readGitInfo(): {sha: string; dirty: boolean} | null {
        // Read .git/HEAD directly to avoid spawning `git`. Resolves
        // symref refs/heads/<branch> -> .git/refs/heads/<branch>.
        const gitDir = path.join(repoRoot, ".git")
        const headPath = path.join(gitDir, "HEAD")
        if (!fs.existsSync(headPath)) return null
        try {
            const head = fs.readFileSync(headPath, "utf8").trim()
            let sha: string
            if (head.startsWith("ref: ")) {
                const refPath = path.join(gitDir, head.slice(5).trim())
                if (!fs.existsSync(refPath)) {
                    // Packed-refs fallback (skip for simplicity).
                    return {sha: head, dirty: false}
                }
                sha = fs.readFileSync(refPath, "utf8").trim()
            } else {
                sha = head
            }
            // We don't try to detect dirty state without spawning git;
            // that would require diffing the index, which is non-
            // trivial. Surface the SHA only.
            return {sha: sha.slice(0, 12), dirty: false}
        } catch {
            return null
        }
    }

    function snapshot(): string {
        const lockPackages = readLockPackages()
        const rows = artifacts.map((a) => {
            const builtAtMs = fs.existsSync(a.artifact)
                ? fs.statSync(a.artifact).mtimeMs
                : 0
            const allDeps = lockPackages
                ? walkLock(a.cargoCrate, lockPackages)
                : null
            // Workspace-internal deps are the ones with no `source =`
            // in the lock — i.e. crates resolved via a `path = `
            // directive somewhere in the workspace. These are the
            // versions you'd actually want to know when asking "which
            // strand is baked into this wasm?" Registry crates
            // (num-bigint, wasm-bindgen, etc.) are summarised as a
            // count to keep the UI readable.
            const internalDeps = allDeps
                ? allDeps.filter((d) => d.internal)
                : null
            const externalDepCount = allDeps
                ? allDeps.filter((d) => !d.internal).length
                : null
            return {
                label: a.label,
                artifactPath: path
                    .relative(pkgs, a.artifact)
                    .replace(/\\/g, "/"),
                builtAt: builtAtMs > 0 ? new Date(builtAtMs).toISOString() : null,
                internalDeps,
                externalDepCount,
            }
        })
        return JSON.stringify(
            {
                generatedAt: new Date().toISOString(),
                git: readGitInfo(),
                artifacts: rows,
            },
            null,
            2
        )
    }

    return {
        name: "workbench-build-info",
        resolveId(id) {
            if (id === VIRTUAL_ID) return RESOLVED_ID
            return null
        },
        load(id) {
            if (id !== RESOLVED_ID) return null
            return `export default ${snapshot()}\n`
        },
        configureServer(server) {
            const watched: string[] = []
            for (const a of artifacts) {
                watched.push(a.artifact)
            }
            // Cargo.lock changes whenever a dep is added/upgraded; we
            // want the build-info card to refresh in that case so the
            // "internal deps" line stays in sync without a restart.
            watched.push(cargoLockPath)
            for (const w of watched) {
                if (fs.existsSync(w)) server.watcher.add(w)
            }
            const invalidate = (): void => {
                const mod = server.moduleGraph.getModuleById(RESOLVED_ID)
                if (mod) server.moduleGraph.invalidateModule(mod)
            }
            const isRelevant = (file: string): boolean => {
                const norm = path.resolve(file)
                return watched.some(
                    (w) => norm === w || norm.startsWith(w + path.sep)
                )
            }
            server.watcher.on("add", (f) => {
                if (isRelevant(f)) invalidate()
            })
            server.watcher.on("change", (f) => {
                if (isRelevant(f)) invalidate()
            })
            server.watcher.on("unlink", (f) => {
                if (isRelevant(f)) invalidate()
            })
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
        keypair?: {pkB64?: unknown; skB64?: unknown}
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
    const kp = s.workbench?.keypair
    if (!kp || typeof kp !== "object") {
        errors.push("workbench.keypair is missing")
        return errors
    }
    if (typeof kp.pkB64 !== "string" || typeof kp.skB64 !== "string") {
        errors.push("workbench.keypair is missing pkB64/skB64 strings")
        return errors
    }
    const ballotStyles = s.state?.ballotStyles ?? {}
    for (const [key, bs] of Object.entries(ballotStyles)) {
        const bsId = bs?.id
        if (typeof bsId !== "string") {
            errors.push(`state.ballotStyles[${key}] has no string id`)
            continue
        }
        const pkInBallotStyle = bs?.ballot_eml?.public_key?.public_key
        if (
            typeof pkInBallotStyle === "string" &&
            pkInBallotStyle !== kp.pkB64
        ) {
            errors.push(
                `state.ballotStyles[${key}].ballot_eml.public_key.public_key does not match workbench.keypair.pkB64`
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
    plugins: [
        react(),
        wasm(),
        topLevelAwait(),
        validateBundledSnapshots(),
        workbenchBuildInfo(),
    ],
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
            // Redirect the `sequent-core` npm name at the freshly built
            // wasm-pack output of the in-tree sequent-core crate
            // (`packages/sequent-core/pkg`). This is a lift-only
            // adaptation: voting-portal's own builds keep using the
            // prebuilt tgz under `voting-portal/rust/`, but the
            // workbench-bundled copy of every booth screen resolves
            // `sequent-core` through this alias and so sees Rust source
            // edits after a manual `yarn build:sequent-core` (the script
            // is opt-in, not chained into `predev`/`prebuild`, so
            // contributors who haven't touched sequent-core Rust pay no
            // toolchain cost). Falls back to the hoisted node_modules
            // copy of the committed tgz if `pkg/` doesn't exist yet.
            // Rationale and trade-offs in LIFTING.md row A7.
            {
                find: /^sequent-core$/,
                replacement: path.resolve(here, "../../sequent-core/pkg"),
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
