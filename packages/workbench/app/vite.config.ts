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
import {execFileSync} from "node:child_process"

const here = path.dirname(fileURLToPath(import.meta.url))
const pkgs = path.resolve(here, "../..")

// wasm-pack output of the in-tree sequent-core crate. Only exists after
// an opt-in `yarn build:sequent-core`; see the `sequent-core` alias.
const sequentCorePkg = path.resolve(here, "../../sequent-core/pkg")

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

    function readGitInfo(): {sha: string} | null {
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
                    return {sha: head}
                }
                sha = fs.readFileSync(refPath, "utf8").trim()
            } else {
                sha = head
            }
            return {sha: sha.slice(0, 12)}
        } catch {
            return null
        }
    }

    /**
     * Spawn `git` with the given args, return stdout as utf-8 or
     * `null` if anything goes wrong (binary missing, non-zero exit,
     * not a git checkout, etc.). Diff-style invocations need a
     * generous buffer cap.
     */
    function runGit(args: string[]): string | null {
        try {
            return execFileSync("git", args, {
                cwd: repoRoot,
                encoding: "utf8",
                maxBuffer: 16 * 1024 * 1024,
                stdio: ["ignore", "pipe", "ignore"],
            })
        } catch {
            return null
        }
    }

    /**
     * The "branch base" commit — i.e. the most-recent ancestor that
     * this branch shares with `origin/main`. Equivalent to GitHub's
     * "X commits ahead of main" semantics. Falls back to whichever
     * upstream ref exists if `origin/main` isn't fetched (e.g. on a
     * branch forked from a release line).
     *
     * On a shallow CI clone the merge-base will resolve to whatever
     * the truncated history terminates at, which is usually wrong;
     * the `baseUnavailableReason` field is populated so the UI can
     * say "shallow clone — base may be misleading" instead of
     * silently showing the wrong commit.
     */
    function readBranchBase(): {
        base: {
            sha: string
            subject: string
            author: string
            date: string
        } | null
        baseUnavailableReason: string | null
    } {
        const candidateRefs = ["origin/main", "origin/master", "main", "master"]
        let baseSha: string | null = null
        let chosenRef: string | null = null
        for (const ref of candidateRefs) {
            const refExists = runGit(["rev-parse", "--verify", "--quiet", ref])
            if (refExists == null) continue
            const merged = runGit(["merge-base", "HEAD", ref])
            if (merged == null) continue
            baseSha = merged.trim()
            chosenRef = ref
            break
        }
        if (baseSha == null) {
            return {
                base: null,
                baseUnavailableReason:
                    "no `origin/main` (or equivalent) ref reachable from " +
                    "this checkout — try `git fetch origin main` and reload",
            }
        }
        const meta = runGit([
            "log",
            "-1",
            "--format=%H%n%s%n%an%n%aI",
            baseSha,
        ])
        if (meta == null) {
            return {
                base: null,
                baseUnavailableReason: `merge-base with \`${chosenRef}\` produced ${baseSha.slice(0, 12)} but \`git log\` could not describe it`,
            }
        }
        const [fullSha, subject, author, date] = meta.split("\n")
        // Shallow-clone heuristic: a single-commit-deep clone has
        // exactly one entry in `git rev-list --all`. We don't gate
        // on it (the data is still useful), just annotate.
        const shallow = fs.existsSync(path.join(repoRoot, ".git/shallow"))
        return {
            base: {
                sha: fullSha.slice(0, 12),
                subject: subject ?? "",
                author: author ?? "",
                date: date ?? "",
            },
            baseUnavailableReason: shallow
                ? "shallow clone — base SHA may be the clone boundary rather than the real fork point"
                : null,
        }
    }

    /**
     * Lifted / shared source drift. For each tracked subtree we emit a
     * unified diff of `HEAD` vs the branch base, so the Diagnostics
     * page answers one question: *what has this branch changed in code
     * it shares with production, and is all of it documented?*
     *
     * The baseline is the merge-base with `origin/main`, so once main
     * is merged into this branch the merge-base advances to the merged
     * commit and each diff collapses to exactly the branch's own
     * edits. A large diff here right after a merge usually means the
     * merge has not been committed yet, not that drift exploded.
     *
     * There is deliberately no "copy lift" flavour any more. The tally
     * components used to be copied out of admin-portal into
     * ui-essentials and were diffed path-against-path at HEAD; that
     * copy was deleted when upstream shipped its own tally
     * visualization, which the workbench now imports unmodified. With
     * nothing copied, git history is the only drift surface left.
     */
    interface DriftTarget {
        label: string
        subtree: string
        /** What the operator should expect to see, rendered as a hint. */
        expectation: string
    }
    const driftTargets: DriftTarget[] = [
        {
            label: "voting-portal/src/",
            subtree: "packages/voting-portal/src/",
            expectation:
                "Section L of LIFTING.md whitelists demo-only concessions " +
                "(ReviewScreen.tsx, castVotesSlice.ts). Anything else here is " +
                "a new concession and needs the doc updated.",
        },
        {
            label: "ui-core/src/",
            subtree: "packages/ui-core/src/",
            expectation:
                "Consumed in place via a Vite alias, never edited. Expected to " +
                "be empty — any diff means the workbench changed a shared library.",
        },
        {
            label: "ui-essentials/src/",
            subtree: "packages/ui-essentials/src/",
            expectation:
                "Consumed in place via a Vite alias, and the tally components are " +
                "imported unmodified. Expected to be empty.",
        },
        {
            label: "velvet + velvet-core",
            subtree: "packages/velvet/",
            expectation:
                "Almost all of this diff is deletions from do_tally — the pure " +
                "tally logic moved *out* into workbench/velvet-core, which " +
                "packages/velvet now re-exports; nothing was lost. The corollary " +
                "is that upstream tally changes must be forward-ported into " +
                "velvet-core rather than merged here, and some currently are not — " +
                "see Known gaps in the workbench README.",
        },
        {
            label: "strand",
            subtree: "packages/strand/",
            expectation:
                "Modified to compile to wasm32; the obsolete openssl/FIPS backends " +
                "were removed rather than feature-gated. The line count looks " +
                "alarming but ~6.4k of the deletions are one file — " +
                "browserstack/package-lock.json — so the actual source change is " +
                "far smaller than the total suggests.",
        },
        {
            label: "sequent-core",
            subtree: "packages/sequent-core/",
            expectation:
                "Shared two ways: as Rust source with velvet-core/velvet-wasm, " +
                "and as a prebuilt tgz with the lifted booth. Expect the wasm32 " +
                "enablement work — dalek pins aligned with strand, `ring` swapped " +
                "for getrandom-0.2/js (ring does not build for " +
                "wasm32-unknown-unknown), web-sys added to the `wasm` feature, and " +
                "the `areas` / `wasm` modules gated on `wasm` instead of " +
                "`wasmtest` so build:sequent-core emits the exports the lifted " +
                "booth calls. That last one widens the crate's public wasm surface " +
                "for every consumer, not just the workbench.",
        },
    ]

    /**
     * Patches are inlined into the virtual module, so an unbounded
     * `git diff` over a tree like strand (thousands of deleted lines)
     * would bloat the dev bundle. Past this many characters we keep
     * the stat and drop the patch, telling the operator which command
     * to run instead.
     */
    const MAX_PATCH_CHARS = 200_000

    function readSourceDrift(baseSha: string | null): Array<{
        label: string
        subtree: string
        expectation: string
        stat: string
        patch: string | null
        patchOmittedReason: string | null
        dirty: boolean
    }> | null {
        if (baseSha == null) return null
        const rows = []
        for (const t of driftTargets) {
            const stat = runGit(["diff", "--stat", baseSha, "--", t.subtree])
            const patch = runGit(["diff", baseSha, "--", t.subtree])
            if (stat == null || patch == null) continue
            const status = runGit(["status", "--porcelain", "--", t.subtree])
            const tooBig = patch.length > MAX_PATCH_CHARS
            rows.push({
                label: t.label,
                subtree: t.subtree,
                expectation: t.expectation,
                stat: stat.trimEnd(),
                patch: tooBig ? null : patch.trimEnd(),
                patchOmittedReason: tooBig
                    ? `diff is ${Math.round(patch.length / 1024)} KB — too large to inline; ` +
                      `run \`git diff ${baseSha.slice(0, 12)} -- ${t.subtree}\``
                    : null,
                dirty: status != null && status.trim().length > 0,
            })
        }
        return rows
    }

    /**
     * How many commits `origin/main` has that HEAD does not. This is
     * the *other* drift axis: the diffs above say what we changed, this
     * says how much upstream has moved since we last merged. `null` if
     * the ref is unreachable.
     */
    function readBehindUpstream(): number | null {
        const out = runGit(["rev-list", "--count", "HEAD..origin/main"])
        if (out == null) return null
        const n = Number.parseInt(out.trim(), 10)
        return Number.isFinite(n) ? n : null
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
        const baseInfo = readBranchBase()
        const git = readGitInfo()
        // The base SHA lookup uses *full* SHAs internally so
        // `git diff <base>` resolves cleanly; the UI receives the
        // short form via `base.sha`. We re-derive the full SHA
        // here for the diff calls.
        const fullBaseSha = baseInfo.base
            ? runGit(["merge-base", "HEAD", "origin/main"])?.trim() ?? null
            : null
        return JSON.stringify(
            {
                generatedAt: new Date().toISOString(),
                git: {
                    sha: git?.sha ?? null,
                    base: baseInfo.base,
                    baseUnavailableReason: baseInfo.baseUnavailableReason,
                    behindUpstream: readBehindUpstream(),
                    sourceDrift: readSourceDrift(fullBaseSha),
                },
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
            // Lifted-source trees: edits here change the drift diffs
            // surfaced on the Diagnostics page, so a re-render is
            // warranted. We watch the source directories; chokidar
            // recurses into them. `.git/HEAD` covers branch switches
            // and commit-on-current-branch (which change the
            // merge-base + voting-portal diff).
            watched.push(path.resolve(repoRoot, "packages/voting-portal/src"))
            watched.push(path.resolve(repoRoot, "packages/ui-core/src"))
            watched.push(path.resolve(repoRoot, "packages/ui-essentials/src"))
            watched.push(path.resolve(repoRoot, ".git/HEAD"))
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
            // toolchain cost). Rationale and trade-offs in LIFTING.md
            // row A7.
            //
            // The alias is registered only when `pkg/` actually exists.
            // `resolve.alias` rewrites unconditionally once registered,
            // so on a fresh clone (where nobody has run
            // `build:sequent-core` yet) an unguarded entry would point
            // every `sequent-core` import at a missing directory and
            // hard-fail with "Failed to resolve import" instead of
            // falling through. Omitting it lets normal node resolution
            // find the hoisted copy of the committed tgz.
            ...(fs.existsSync(sequentCorePkg)
                ? [
                      {
                          find: /^sequent-core$/,
                          replacement: sequentCorePkg,
                      },
                  ]
                : []),
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
