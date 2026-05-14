// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Workbench-only Redux persistence layer.
//
// Mirrors the entire voting-portal Redux state to `localStorage` on every
// dispatch, and rehydrates from it on app boot. The result is auto-resume:
// the workbench picks up exactly where the previous tab left off, even
// after a tab close, browser restart, or dev-server restart.
//
// Why subscribe + replay rather than `redux-persist` or a root-reducer
// wrapper:
//   - We MUST NOT modify `voting-portal/src/store/store.ts`. The store
//     is already constructed with a fixed reducer when this code runs;
//     we cannot inject `preloadedState` after the fact.
//   - `store.subscribe(...)` is the public API for observing state
//     changes from the outside, which is exactly the seam we have.
//   - Rehydration replays the existing `setX` action creators
//     ("setElection", "setBallotStyle", etc.) per persisted entity. This
//     keeps the workbench honest: if voting-portal renames an action or
//     changes a payload shape, this file fails to type-check at the
//     dispatch site, telling us exactly what to update.
//
// Slices we currently rehydrate:
//   elections, electionEvent, ballotStyles, ballotSelections, castVotes,
//   extra (bypassChooser + isVoted).
//
// Slices we deliberately skip (rehydration becomes a TODO when a screen
// that consumes them is lifted):
//   supportMaterials, documents, auditableBallots, confirmationScreenData.
// They will simply be empty after a reload until then; that matches the
// current state of the workbench, so nothing visible regresses.
//
// See `LIFTING.md` section J ("Persistence + snapshots") for the
// architectural rationale and the canary table.

import type {RootState, store as Store} from "voting-portal/src/store/store"
import {setElection} from "voting-portal/src/store/elections/electionsSlice"
import {setElectionEvent} from "voting-portal/src/store/electionEvents/electionEventsSlice"
import {setBallotStyle} from "voting-portal/src/store/ballotStyles/ballotStylesSlice"
import {
    setBallotSelection,
    resetBallotSelection,
} from "voting-portal/src/store/ballotSelections/ballotSelectionsSlice"
import {addCastVotes} from "voting-portal/src/store/castVotes/castVotesSlice"
import {
    setBypassChooser,
    setIsVoted,
} from "voting-portal/src/store/extra/extraSlice"

/**
 * Storage key. The `:v1` suffix is a schema version: when the persisted
 * shape becomes incompatible (e.g. voting-portal removes a slice we
 * relied on), bump the suffix and the old data is silently ignored on
 * the next boot rather than crashing on a missing reducer.
 */
export const PERSISTENCE_KEY = "workbench:state:v1"

// We disable the writer during hydration so that the per-slice
// dispatches we issue while restoring don't repeatedly overwrite
// `localStorage` with intermediate partial states.
let suspendWrites = false

export interface PersistedSnapshot {
    /** Schema version baked into the JSON itself. Must match
     *  `PERSISTENCE_KEY`'s suffix or the snapshot is rejected. */
    version: "v1"
    state: RootState
}

/**
 * Read the persisted snapshot. Returns `null` on first boot, on schema
 * mismatch, or on parse failure — callers should fall back to seeding
 * fresh fixtures in those cases.
 */
export function loadPersistedSnapshot(): PersistedSnapshot | null {
    if (typeof localStorage === "undefined") return null
    const raw = localStorage.getItem(PERSISTENCE_KEY)
    if (!raw) return null
    try {
        const parsed = JSON.parse(raw) as PersistedSnapshot
        if (parsed.version !== "v1") {
            console.warn(
                `[workbench/persistence] discarding snapshot with version ${String(
                    parsed.version
                )}; expected v1`
            )
            return null
        }
        return parsed
    } catch (e) {
        console.warn("[workbench/persistence] failed to parse snapshot:", e)
        return null
    }
}

/**
 * Replace the live store with the contents of a persisted snapshot.
 *
 * We dispatch the portal's own action creators per entity so the live
 * state is built up by reducers we know are correct, rather than by
 * patching state directly. Order matters: ballotStyles must be present
 * before ballotSelections, because `setBallotSelection` reads the
 * current entry to decide whether to apply the update.
 */
export function hydrateFromSnapshot(
    store: typeof Store,
    snapshot: PersistedSnapshot
): void {
    const {state} = snapshot
    suspendWrites = true
    try {
        for (const election of Object.values(state.elections)) {
            if (election) store.dispatch(setElection(election))
        }
        for (const event of Object.values(state.electionEvent)) {
            if (event) store.dispatch(setElectionEvent(event))
        }
        for (const ballotStyle of Object.values(state.ballotStyles)) {
            if (ballotStyle) store.dispatch(setBallotStyle(ballotStyle))
        }
        for (const [electionId, selection] of Object.entries(
            state.ballotSelections
        )) {
            if (!selection) continue
            const ballotStyle = Object.values(state.ballotStyles).find(
                (bs) => bs?.election_id === electionId
            )
            if (!ballotStyle) continue
            // Initialise the per-election entry first so the
            // `setBallotSelection` reducer (which is a no-op when the
            // entry is absent) actually accepts the payload.
            store.dispatch(resetBallotSelection({ballotStyle, force: true}))
            store.dispatch(
                setBallotSelection({ballotStyle, ballotSelection: selection})
            )
        }
        for (const [, votes] of Object.entries(state.castVotes)) {
            if (votes && votes.length > 0) store.dispatch(addCastVotes(votes))
        }
        if (state.extra) {
            if (typeof state.extra.bypassChooser === "boolean") {
                store.dispatch(setBypassChooser(state.extra.bypassChooser))
            }
            if (typeof state.extra.isVoted === "boolean") {
                store.dispatch(setIsVoted(state.extra.isVoted))
            }
        }
    } finally {
        suspendWrites = false
        // Force one persisted write so the post-hydration state is what
        // sits in localStorage from this point on.
        writeSnapshot(store.getState())
    }
}

function writeSnapshot(state: RootState): void {
    if (typeof localStorage === "undefined") return
    const snapshot: PersistedSnapshot = {version: "v1", state}
    try {
        localStorage.setItem(PERSISTENCE_KEY, JSON.stringify(snapshot))
    } catch (e) {
        // Most likely cause: quota exceeded. We don't want to crash the
        // app over persistence; warn and continue. Snapshot promotion
        // (saving named checkpoints to disk) is the long-term answer.
        console.warn("[workbench/persistence] write failed:", e)
    }
}

/**
 * Subscribe to store changes and persist on every dispatch.
 *
 * Returns the unsubscribe function for completeness; in practice we
 * never tear down the subscription because the store outlives the
 * workbench process.
 */
export function installPersistence(store: typeof Store): () => void {
    return store.subscribe(() => {
        if (suspendWrites) return
        writeSnapshot(store.getState())
    })
}

/**
 * Wipe all persisted workbench state. After calling this you typically
 * want to reload the page so the boot path takes the "fresh fixtures"
 * branch again.
 */
export function clearPersistedSnapshot(): void {
    if (typeof localStorage === "undefined") return
    localStorage.removeItem(PERSISTENCE_KEY)
}
