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
    setRepairedDecodedBigInts,
    subscribeWorkbench,
    type RepairedCastVote,
    type WorkbenchExtraState,
} from "./workbenchStore"
import {decryptBallotContent} from "./tally"
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

// Provenance: the id of the snapshot the current working copy was
// forked off of. Set by `hydrateFromSnapshot` to the `sourceId`
// passed in (or, on warm boot, recovered from the persisted snapshot
// itself). Baked into every subsequent write so checkpoints and the
// auto-resume slot remember their lineage. Tagged-id scheme:
//
//   bundled:<filename>     — a JSON shipped in src/fixtures/snapshots/
//   checkpoint:<name>      — a localStorage checkpoint saved by the
//                            operator
//
// `null` means the snapshot is a root (no parent) — currently only
// the bundled `default.json` is a root.
let currentParentId: string | null = null

/** The id of the snapshot the live working copy was forked off of, or
 *  `null` if the working copy is a root. Reads from a module-level
 *  cache that `hydrateFromSnapshot` keeps in sync; cheap and safe to
 *  call from React renders. */
export function getCurrentParentId(): string | null {
    return currentParentId
}

/** Build a tagged id for a bundled snapshot. */
export const bundledId = (name: string): string => `bundled:${name}`

/** Build a tagged id for a named checkpoint. */
export const checkpointId = (name: string): string => `checkpoint:${name}`

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
    /** Tagged id of the snapshot this one was forked from, or `null`
     *  for a root. Carried on bundled JSONs, named checkpoints, and
     *  the auto-resume slot alike. Optional so snapshots written
     *  before provenance was added still load (they hydrate as if
     *  `parentId === null`). See {@link currentParentId}. */
    parentId?: string | null
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
 *
 * `sourceId` records what the resulting working copy was forked off
 * of. Callers should pass a tagged id (see {@link bundledId} /
 * {@link checkpointId}). When omitted, we recover `currentParentId`
 * from the snapshot's own `parentId` — the correct behaviour for
 * warm boots that replay the auto-resume slot, since the auto-resume
 * slot persists its parent-id across reloads.
 */
export function hydrateFromSnapshot(
    store: typeof Store,
    snapshot: PersistedSnapshot,
    sourceId?: string | null
): void {
    const {state, workbench} = snapshot
    currentParentId =
        sourceId !== undefined ? sourceId : snapshot.parentId ?? null
    suspendWrites = true
    try {
        // Workbench overlay state is restored FIRST so that any cast
        // votes we replay below land in a store whose attribution
        // ledger already knows about them. (In practice the ledger is
        // the source of truth on its own, and replayed votes are
        // already present in the ledger from the snapshot itself, but
        // ordering this way keeps the invariant clean.)
        replaceWorkbenchState(
            workbench ?? {
                voters: [],
                activeVoterId: null,
                castBy: {},
                repairedCastVotes: {},
                keypairs: {},
            }
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
        parentId: currentParentId,
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
 * overlay.
 *
 * The encrypted ballot is already on the cast-vote record itself
 * (`cv.content`, set by the demo helper in voting-portal —
 * see LIFTING.md section L), so the bridge only has to capture the
 * one thing production discards: the **plaintext selection** from
 * `state.ballotSelections`. That gives the workbench a human-readable
 * view of what the voter chose, independent of decryption keys, and
 * is the input a future inline tally will encode + tally via
 * velvet-wasm.
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
    castVote: {id: string; election_id?: string | null; content?: string | null}
): void {
    const castVoteElectionId = castVote.election_id
    if (!castVoteElectionId) return
    const ballotStyle = Object.values(rootState.ballotStyles).find(
        (bs) => bs && bs.election_id === castVoteElectionId
    )
    if (!ballotStyle) return

    const selection = rootState.ballotSelections[castVoteElectionId]
    if (!selection) return

    const repaired: RepairedCastVote = {
        electionId: castVoteElectionId,
        ballotStyleId: ballotStyle.id,
        // Deep-clone via JSON round-trip so future Redux dispatches
        // that mutate `state.ballotSelections` in place cannot affect
        // the snapshot we just took.
        selection: JSON.parse(JSON.stringify(selection)) as unknown,
        // Filled in asynchronously below. Captured empty up-front so
        // the UI can render the row immediately; the decoded BigUints
        // appear a moment later via `setRepairedDecodedBigInts`.
        decodedBigInts: {},
        capturedAt: new Date().toISOString(),
    }
    captureRepairedCastVote(castVote.id, repaired)

    // Async-decrypt every contest on this ballot style using the
    // workbench-owned secret key for THIS ballot style. Fire-and-
    // forget: failure to decrypt a contest leaves its entry out of
    // `decodedBigInts`, and the tally surfaces that as `no-data`. We
    // deliberately do not await here — the store subscriber is sync,
    // and the cast-vote row is already useful with the plaintext
    // selection alone.
    const keypair = getWorkbenchState().keypairs[ballotStyle.id]
    if (!keypair || !castVote.content) return
    const content = castVote.content
    const contestIds = ballotStyle.ballot_eml.contests.map((c) => c.id)
    void (async () => {
        const decoded: Record<string, string> = {}
        for (const contestId of contestIds) {
            try {
                decoded[contestId] = await decryptBallotContent(
                    content,
                    keypair.skB64,
                    contestId
                )
            } catch (e) {
                // Best-effort: log once and continue. A failed
                // decrypt typically means the cast vote was encrypted
                // under a different keypair (e.g. snapshot whose
                // ballot style was re-keyed without re-encrypting
                // prior votes).
                console.warn(
                    `[workbench/persistence] decrypt failed for cv=${castVote.id} contest=${contestId}:`,
                    e
                )
            }
        }
        if (Object.keys(decoded).length > 0) {
            setRepairedDecodedBigInts(castVote.id, decoded)
        }
    })()
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
                tryCaptureRepairedCastVote(liveState, v)
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
    /** Tagged id of the snapshot this checkpoint was forked from
     *  (`bundled:<name>` or `checkpoint:<name>`), or `null` for a
     *  root. Persisted in the index so the inspector can render the
     *  provenance forest without reading every blob.
     *
     *  Older checkpoint indices (pre-task-5) did not carry this
     *  field; we treat the absence as "unknown parent", which the
     *  inspector surfaces under the Detached group until the
     *  operator re-saves. */
    parentId?: string | null
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
        ).map((e) => ({
            // Normalise legacy entries (missing parentId) to `undefined`
            // so callers can distinguish "unknown" (legacy) from
            // "explicit root" (null).
            name: e.name,
            savedAt: e.savedAt,
            parentId: (e as CheckpointMeta).parentId,
        }))
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
    // A checkpoint inherits the working copy's current parent. After
    // it is saved, the working copy is conceptually a fork of the
    // new checkpoint — we reflect that by retargeting
    // `currentParentId` and forcing a write so the auto-resume slot
    // picks up the new lineage immediately.
    const snapshot: PersistedSnapshot = {
        version: "v1",
        state: store.getState(),
        workbench: getWorkbenchState(),
        parentId: currentParentId,
    }
    if (typeof localStorage === "undefined") {
        throw new Error("Cannot save checkpoint: localStorage is unavailable.")
    }
    localStorage.setItem(CHECKPOINT_PREFIX + name, JSON.stringify(snapshot))

    const meta: CheckpointMeta = {
        name,
        savedAt: new Date().toISOString(),
        parentId: currentParentId,
    }
    const next = readCheckpointIndex().filter((e) => e.name !== name)
    next.push(meta)
    writeCheckpointIndex(next)

    currentParentId = checkpointId(name)
    writeSnapshot(store.getState())
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
    hydrateFromSnapshot(store, snapshot, checkpointId(name))
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
