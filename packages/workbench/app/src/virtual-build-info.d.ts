// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Ambient type for the `virtual:workbench-build-info` module supplied
// by the `workbenchBuildInfo` Vite plugin in `../vite.config.ts`. The
// plugin emits a JSON snapshot of wasm-artifact mtimes vs. their
// crate-source mtimes so the homepage can surface "is the wasm I'm
// looking at older than the Rust source" without contributors
// re-running `cargo build` to find out.
declare module "virtual:workbench-build-info" {
    export interface WorkbenchBuildDep {
        /** Crate name as it appears in `[[package]].name` in Cargo.lock. */
        name: string
        /** Resolved version from Cargo.lock. */
        version: string
        /**
         * `true` for workspace-internal crates (path-deps with no
         * `source =` field in Cargo.lock). `false` for registry / git
         * deps. The UI typically only lists the internal ones.
         */
        internal: boolean
    }
    export interface WorkbenchBuildArtifact {
        label: string
        /** Workspace-relative POSIX path to the wasm artifact. */
        artifactPath: string
        /** ISO 8601 mtime of the artifact, or `null` if it doesn't exist. */
        builtAt: string | null
        /**
         * Workspace-internal crates baked into this wasm, with their
         * resolved versions (BFS from the crate's lock entry). `null`
         * if Cargo.lock could not be parsed.
         */
        internalDeps: WorkbenchBuildDep[] | null
        /** Count of transitive registry / git deps. `null` on parse failure. */
        externalDepCount: number | null
    }
    export interface WorkbenchBuildBase {
        /** Short (12-char) SHA of the branch base commit. */
        sha: string
        /** Subject line of the base commit. */
        subject: string
        /** Author name of the base commit. */
        author: string
        /** ISO 8601 author date of the base commit. */
        date: string
    }
    export interface WorkbenchVotingPortalDiff {
        /** `git diff --stat` output (short summary). */
        stat: string
        /** Full unified diff (may be multi-KB; render in a collapsed block). */
        patch: string
        /**
         * `true` if there are uncommitted changes under
         * `packages/voting-portal/`. Treat the patch above as
         * committed-only and warn the operator that working-tree
         * edits aren't reflected.
         */
        dirty: boolean
    }
    export interface WorkbenchTallyLiftDiff {
        /** Human label for the pair (filename + LIFTING-TALLY adaptation ids). */
        label: string
        /**
         * `modified` — both files exist; `patch` is a unified diff.
         * `added`    — only the lifted copy exists; `patch` is null.
         */
        kind: "modified" | "added"
        /** Workspace-relative path to the admin-portal original. */
        origPath: string | null
        /** Workspace-relative path to the ui-essentials lifted copy. */
        copyPath: string
        /** `git diff --stat` style summary (or size for added files). */
        stat: string | null
        /** Full unified diff, or `null` for added files / missing originals. */
        patch: string | null
        /** Operator-facing note when a pair could not be diffed. */
        note: string | null
    }
    export interface WorkbenchGitInfo {
        /** Short git SHA of HEAD. `null` if the .git/HEAD read failed. */
        sha: string | null
        /**
         * Branch base — the most-recent ancestor shared with
         * `origin/main` (or the first reachable fallback ref). `null`
         * if no upstream ref could be found; check
         * `baseUnavailableReason` for the operator-facing reason.
         */
        base: WorkbenchBuildBase | null
        /**
         * `null` when the base lookup succeeded; otherwise a
         * human-readable string suitable for inline rendering.
         */
        baseUnavailableReason: string | null
        /**
         * `voting-portal/src/` drift relative to {@link base}. `null`
         * when the base lookup failed.
         */
        votingPortalDiff: WorkbenchVotingPortalDiff | null
        /**
         * Tally-lift drift — per-file diffs between admin-portal
         * originals and the ui-essentials copies, plus listings for
         * lifted-only "added" files. Always an array (possibly
         * with `note` set on individual rows when a file moved).
         */
        tallyLiftDiffs: WorkbenchTallyLiftDiff[]
    }
    export interface WorkbenchBuildInfo {
        /** ISO 8601 timestamp of when this module was loaded by Vite. */
        generatedAt: string
        /**
         * Git provenance + lifted-source drift. Always non-null in
         * a git checkout (degrades gracefully via the nullable
         * sub-fields when individual probes fail).
         */
        git: WorkbenchGitInfo
        artifacts: WorkbenchBuildArtifact[]
    }
    const info: WorkbenchBuildInfo
    export default info
}
