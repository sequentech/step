// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Bundled-snapshot loader.
//
// Every JSON file in `./snapshots/` is eagerly imported at build time
// via Vite's `import.meta.glob`. Each one is a full {@link
// PersistedSnapshot} produced exactly the same way an auto-resume
// write produces one — there is no second "fixture" format. The
// snapshot is the unit of state.
//
// Why bundled-at-build-time rather than fetched at runtime:
//   - Snapshots become part of the build artifact, versioned with the
//     code that consumes them.
//   - A first-boot fallback (when nothing is in `localStorage`) is a
//     single dictionary lookup, no async.
//   - The build-time validation test (see step-6 plan, task 4) can
//     iterate the same dictionary and reject snapshots whose ballot
//     styles' `ballot_eml.public_key.public_key` does not match
//     `workbench.keypair.pkB64`.
//
// To add a new bundled snapshot: drop a JSON file in `./snapshots/`
// and a matching `.json.license` sidecar. The filename (without the
// extension) is the snapshot's id, used as the bundled-source tag
// when provenance is recorded on a loaded scenario.

import type {PersistedSnapshot} from "../persistence"

// `import.meta.glob` rewrites this call at build time into a static
// dictionary of {path -> module}; the runtime never touches the
// filesystem. `eager: true` inlines the JSON; `import: "default"`
// gives us the parsed object rather than the module wrapper.
const modules = import.meta.glob<PersistedSnapshot>(
    "./snapshots/*.json",
    {eager: true, import: "default"}
)

/** Map from snapshot id (filename without `.json`) to its parsed
 *  payload. Built once at module load; subsequent lookups are O(1). */
export const BUNDLED_SNAPSHOTS: Record<string, PersistedSnapshot> = (() => {
    const out: Record<string, PersistedSnapshot> = {}
    for (const [path, snapshot] of Object.entries(modules)) {
        // Path looks like "./snapshots/default.json"; we want
        // "default" as the id.
        const m = path.match(/\/([^/]+)\.json$/)
        if (!m) continue
        out[m[1]] = snapshot
    }
    return out
})()

/** Look up a bundled snapshot by id. Returns `null` if no such id
 *  exists; callers can use that to fall through to a different
 *  default or surface a tree-rail error state. */
export function loadBundledSnapshot(id: string): PersistedSnapshot | null {
    return BUNDLED_SNAPSHOTS[id] ?? null
}

// --- Session-only mutation ------------------------------------------------
//
// The map above is built once at module load from the static
// `import.meta.glob` dictionary. Operators can delete an entry at
// runtime via the inspector's Snapshots table — that mutation lives
// only in this module's memory (it does not touch the build artifact
// on disk), and a page reload restores every bundled snapshot.
// A tiny pub-sub lets React views re-render when the set changes
// without prop-drilling.

const listeners = new Set<() => void>()

/** Subscribe to bundled-snapshot mutations. Returns an unsubscribe
 *  callback suitable for `useSyncExternalStore`. */
export function subscribeBundledSnapshots(cb: () => void): () => void {
    listeners.add(cb)
    return () => listeners.delete(cb)
}

/** Remove a bundled snapshot from the in-memory dictionary. Session
 *  only — the source JSON on disk is untouched and the entry returns
 *  on the next page reload. No-op if the id is unknown. */
export function deleteBundledSnapshot(id: string): void {
    if (!(id in BUNDLED_SNAPSHOTS)) return
    delete BUNDLED_SNAPSHOTS[id]
    for (const cb of listeners) cb()
}
