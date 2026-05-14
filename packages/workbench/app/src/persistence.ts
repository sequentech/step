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
import {
    attributeCastVote,
    captureRepairedCastVote,
    getWorkbenchState,
    replaceWorkbenchState,
    subscribeWorkbench,
    type RepairedCastVote,
    type WorkbenchExtraState,
} from "./workbenchStore"
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
    /** Workbench-only overlay state (voter directory, attribution
     *  ledger). Optional so snapshots written before this field was
     *  added still load — they just rehydrate with an empty workbench
     *  state. New snapshots always include it. */
    workbench?: WorkbenchExtraState
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
    const {state, workbench} = snapshot
    suspendWrites = true
    try {
        // Workbench overlay state is restored FIRST so that any cast
        // votes we replay below land in a store whose attribution
        // ledger already knows about them. (In practice the ledger is
        // the source of truth on its own, and replayed votes are
        // already present in the ledger from the snapshot itself, but
        // ordering this way keeps the invariant clean.)
        replaceWorkbenchState(
            workbench ?? {voters: [], activeVoterId: null, castBy: {}}
        )
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
    const snapshot: PersistedSnapshot = {
        version: "v1",
        state,
        workbench: getWorkbenchState(),
    }
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
 * Workbench bridge: turn a freshly-observed cast vote into a
 * {@link RepairedCastVote} stored alongside it in the workbench
 * overlay. The fields we collect compensate for what the demo cast-
 * vote record drops on the floor:
 *
 *   - **Plaintext selection.** Snapshotted from
 *     `state.ballotSelections[cv.election_id]`, which holds the user's
 *     actual choices at cast time. This is the input a future inline
 *     tally will encode + tally via velvet-wasm. The cast-vote record
 *     itself carries only an empty `content` field on the demo path.
 *   - **Encrypted hashable ballot.** Read from
 *     `sessionStorage["ballotData"].ballot` if present. Display-only:
 *     the workbench has no decryption keys, so this string is opaque.
 *
 * `electionId` and `ballotStyleId` are recorded on the snapshot for
 * convenience; they're discovered by looking up the matching ballot
 * style by `cv.election_id`.
 *
 * Best-effort: if any source is missing (e.g. the ballot style isn't
 * in Redux), the function silently records nothing. Subsequent votes
 * are independent and may succeed.
 */
function tryCaptureRepairedCastVote(
    rootState: RootState,
    castVoteId: string,
    castVoteElectionId: string | null | undefined
): void {
    if (!castVoteElectionId) return
    const ballotStyle = Object.values(rootState.ballotStyles).find(
        (bs) => bs && bs.election_id === castVoteElectionId
    )
    if (!ballotStyle) return

    const selection = rootState.ballotSelections[castVoteElectionId]
    if (!selection) return

    // sessionStorage["ballotData"] is set by ReviewScreen's
    // `storeBallotDataAndReauth` immediately before the cast vote is
    // dispatched (see voting-portal/src/routes/ReviewScreen.tsx). It
    // is the encrypted hashable ballot the booth would have sent to
    // the backend in a non-demo run.
    let hashableBallotJson: string | null = null
    if (typeof sessionStorage !== "undefined") {
        try {
            const raw = sessionStorage.getItem("ballotData")
            if (raw) {
                const parsed = JSON.parse(raw) as {ballot?: unknown}
                if (typeof parsed.ballot === "string") {
                    hashableBallotJson = parsed.ballot
                }
            }
        } catch {
            // sessionStorage is best-effort; swallow parse errors.
        }
    }

    const repaired: RepairedCastVote = {
        electionId: castVoteElectionId,
        ballotStyleId: ballotStyle.id,
        // Deep-clone via JSON round-trip so future Redux dispatches
        // that mutate `state.ballotSelections` in place cannot affect
        // the snapshot we just took.
        selection: JSON.parse(JSON.stringify(selection)) as unknown,
        hashableBallotJson,
        capturedAt: new Date().toISOString(),
    }
    captureRepairedCastVote(castVoteId, repaired)
}

/**
 * Subscribe to store changes and persist on every dispatch.
 *
 * Returns the unsubscribe function for completeness; in practice we
 * never tear down the subscription because the store outlives the
 * workbench process.
 */
export function installPersistence(store: typeof Store): () => void {
    // Track previously-seen cast-vote ids so we can attribute only the
    // new ones to the currently-active voter. Initialised from
    // whatever's already in the store at install time so a hydrated
    // boot doesn't double-attribute every existing vote.
    const seenCastVoteIds = new Set<string>()
    for (const votes of Object.values(store.getState().castVotes)) {
        if (!votes) continue
        for (const v of votes) seenCastVoteIds.add(v.id)
    }

    const unsubStore = store.subscribe(() => {
        if (suspendWrites) {
            // Even though we don't write, we still need to keep the
            // "seen" set in sync with state replays so hydration doesn't
            // leave us thinking every restored vote is new.
            for (const votes of Object.values(store.getState().castVotes)) {
                if (!votes) continue
                for (const v of votes) seenCastVoteIds.add(v.id)
            }
            return
        }
        // Detect newly-arrived cast votes and bridge them into the
        // workbench overlay. Two captures happen per new vote:
        //   1. attributeCastVote(): tags the vote with the active voter
        //      (no-op when no voter is active).
        //   2. captureRepairedCastVote(): snapshots the data the demo
        //      path doesn't put on the cast-vote record itself — the
        //      plaintext selection (from state.ballotSelections) and
        //      the encrypted hashable ballot (from
        //      sessionStorage["ballotData"]).
        const liveState = store.getState()
        for (const votes of Object.values(liveState.castVotes)) {
            if (!votes) continue
            for (const v of votes) {
                if (seenCastVoteIds.has(v.id)) continue
                seenCastVoteIds.add(v.id)
                attributeCastVote(v.id)
                tryCaptureRepairedCastVote(liveState, v.id, v.election_id)
            }
        }
        writeSnapshot(store.getState())
    })

    // Workbench-overlay changes (adding a voter, switching active voter)
    // must also flow into the auto-resume slot, otherwise they would be
    // lost on reload. The workbench mini-store fires its listener after
    // every mutation; we just rewrite the snapshot in response.
    const unsubWorkbench = subscribeWorkbench(() => {
        if (suspendWrites) return
        writeSnapshot(store.getState())
    })

    return () => {
        unsubStore()
        unsubWorkbench()
    }
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

// ---------------------------------------------------------------------------
// Named checkpoints
// ---------------------------------------------------------------------------
//
// In addition to the single auto-resume slot above (`workbench:state:v1`),
// the operator can save the current Redux state under a human-chosen name
// and restore it later. Mental model — three independent tiers:
//
//   1. Auto-resume slot (`workbench:state:v1`).
//      Overwritten on every dispatch. Survives reloads. Wiped by the
//      "Reset workbench state" button.
//   2. Named checkpoint (`workbench:checkpoint:v1:<name>`).
//      Snapshotted explicitly by the operator. Independent of the
//      auto-resume slot — saving doesn't pause auto-resume, loading
//      overwrites the auto-resume slot via hydrateFromSnapshot's tail
//      writeSnapshot call.
//   3. Bundled fixture (in-repo, future).
//      Compiled into the workbench bundle; not implemented yet.
//
// Storage scheme:
//   workbench:checkpoints:v1     -> JSON array of `CheckpointMeta` (the
//                                   index; canonical list of names).
//   workbench:checkpoint:v1:<n>  -> the serialized `PersistedSnapshot`
//                                   for checkpoint named <n>.
//
// We keep an index rather than scanning `localStorage` so that we can
// surface metadata (savedAt, optional notes) in the UI without parsing
// every snapshot.

const CHECKPOINT_INDEX_KEY = "workbench:checkpoints:v1"
const CHECKPOINT_PREFIX = "workbench:checkpoint:v1:"

export interface CheckpointMeta {
    /** Human-chosen identifier. Doubles as the localStorage suffix, so
     *  it must round-trip through `localStorage.getItem`. We restrict
     *  the charset in {@link normalizeCheckpointName} to keep the key
     *  predictable and copy-pasteable. */
    name: string
    /** ISO-8601 timestamp of when this checkpoint was last saved. */
    savedAt: string
}

/**
 * Trim and validate a user-supplied checkpoint name. We allow letters,
 * digits, dash, underscore, dot, and space — and cap at 64 chars so the
 * resulting key stays well inside any practical localStorage limit.
 *
 * Throws on empty / oversize / illegal-charset input rather than
 * silently normalising, so the UI can surface a precise error.
 */
export function normalizeCheckpointName(raw: string): string {
    const trimmed = raw.trim()
    if (trimmed.length === 0) {
        throw new Error("Checkpoint name must not be empty.")
    }
    if (trimmed.length > 64) {
        throw new Error("Checkpoint name must be 64 characters or fewer.")
    }
    if (!/^[\w. -]+$/.test(trimmed)) {
        throw new Error(
            "Checkpoint name may contain only letters, digits, spaces, '.', '-', and '_'."
        )
    }
    return trimmed
}

function readCheckpointIndex(): CheckpointMeta[] {
    if (typeof localStorage === "undefined") return []
    const raw = localStorage.getItem(CHECKPOINT_INDEX_KEY)
    if (!raw) return []
    try {
        const parsed = JSON.parse(raw) as unknown
        if (!Array.isArray(parsed)) return []
        return parsed.filter(
            (e): e is CheckpointMeta =>
                !!e &&
                typeof (e as CheckpointMeta).name === "string" &&
                typeof (e as CheckpointMeta).savedAt === "string"
        )
    } catch {
        return []
    }
}

function writeCheckpointIndex(index: CheckpointMeta[]): void {
    if (typeof localStorage === "undefined") return
    // Keep the index sorted by name for stable UI ordering.
    const sorted = [...index].sort((a, b) =>
        a.name.localeCompare(b.name, undefined, {sensitivity: "base"})
    )
    localStorage.setItem(CHECKPOINT_INDEX_KEY, JSON.stringify(sorted))
}

/**
 * Return the list of saved checkpoints. Cheap — only reads the index,
 * not the snapshots themselves.
 */
export function listCheckpoints(): CheckpointMeta[] {
    return readCheckpointIndex()
}

/**
 * Save the current store state as a named checkpoint. Overwrites a
 * previous checkpoint with the same (normalized) name.
 *
 * Returns the metadata that was written, which is useful for tests and
 * for the UI to update its row immediately without re-reading the
 * index.
 */
export function saveCheckpoint(
    store: typeof Store,
    rawName: string
): CheckpointMeta {
    const name = normalizeCheckpointName(rawName)
    const snapshot: PersistedSnapshot = {version: "v1", state: store.getState()}
    if (typeof localStorage === "undefined") {
        throw new Error("Cannot save checkpoint: localStorage is unavailable.")
    }
    localStorage.setItem(CHECKPOINT_PREFIX + name, JSON.stringify(snapshot))

    const meta: CheckpointMeta = {name, savedAt: new Date().toISOString()}
    const next = readCheckpointIndex().filter((e) => e.name !== name)
    next.push(meta)
    writeCheckpointIndex(next)
    return meta
}

/**
 * Hydrate the live store from a saved checkpoint. The auto-resume slot
 * is overwritten as a side-effect (via the writeSnapshot call inside
 * {@link hydrateFromSnapshot}), so a subsequent reload picks up the
 * loaded state as the new baseline.
 *
 * Returns `true` on success, `false` if no such checkpoint exists.
 */
export function loadCheckpoint(store: typeof Store, rawName: string): boolean {
    const name = normalizeCheckpointName(rawName)
    if (typeof localStorage === "undefined") return false
    const raw = localStorage.getItem(CHECKPOINT_PREFIX + name)
    if (!raw) return false
    let snapshot: PersistedSnapshot
    try {
        snapshot = JSON.parse(raw) as PersistedSnapshot
    } catch (e) {
        console.warn(
            `[workbench/persistence] failed to parse checkpoint "${name}":`,
            e
        )
        return false
    }
    if (snapshot.version !== "v1") return false
    hydrateFromSnapshot(store, snapshot)
    return true
}

/**
 * Delete a saved checkpoint. No-op if no checkpoint by that name
 * exists.
 */
export function deleteCheckpoint(rawName: string): void {
    const name = normalizeCheckpointName(rawName)
    if (typeof localStorage === "undefined") return
    localStorage.removeItem(CHECKPOINT_PREFIX + name)
    writeCheckpointIndex(
        readCheckpointIndex().filter((e) => e.name !== name)
    )
}
