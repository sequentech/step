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
    export interface WorkbenchBuildInfo {
        /** ISO 8601 timestamp of when this module was loaded by Vite. */
        generatedAt: string
        /** Short git SHA from `.git/HEAD`. `null` if not in a git repo. */
        git: {sha: string} | null
        artifacts: WorkbenchBuildArtifact[]
    }
    const info: WorkbenchBuildInfo
    export default info
}
