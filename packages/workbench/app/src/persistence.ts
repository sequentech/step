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
// What we persist:
//
// The workbench saves only the *canonical scenario state* — the data
// that, if absent on reload, would make the scenario look or behave
// differently to a workbench user. Concretely:
//
//   Redux: elections, electionEvent, ballotStyles, ballotSelections,
//          castVotes, extra (bypassChooser + isVoted).
//   Workbench overlay: voters, activeVoterId, castBy, repairedCastVotes,
//                      keypair.
//
// Everything else the voting-portal store carries (auditableBallots,
// supportMaterials, documents, confirmationScreenData) is
// booth-internal scratch / cache. The encrypted ballot payload that
// matters for tally lives on each `castVotes[*].content`, so dropping
// the redundant copy in `auditableBallots` is lossless for the
// workbench's purposes. Same for the others — they are either
// re-derived on next booth interaction or refetched from the backend.
//
// Consequence: byte-equality between the live canonical projection
// and a saved blob's `state` is also the *semantic* notion of
// "unchanged scenario", which is what the dirty indicator wants to
// answer.
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
import {addCastVotes, removeCastVotes} from "voting-portal/src/store/castVotes/castVotesSlice"
import {
    setBypassChooser,
    setIsVoted,
    clearIsVoted,
} from "voting-portal/src/store/extra/extraSlice"

import {
    attributeCastVote,
    captureRepairedCastVote,
    dropCastVoteOverlay,
    getWorkbenchState,
    replaceWorkbenchState,
    selectBallotStyleForVoter,
    setActiveVoter,
    setRepairedDecodedBigInts,
    subscribeWorkbench,
    type RepairedCastVote,
    type WorkbenchExtraState,
} from "./workbenchStore"
import {decryptBallotContent} from "./tally"
import {loadBundledSnapshot} from "./fixtures/bundledSnapshots"
import {
    applyPolicyOverlayToBallotStyleRow,
    getPolicyOverrides,
    subscribePolicyOverrides,
} from "./policyOverridesStore"
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
    /** Canonical scenario state. A {@link projectCanonicalState}
     *  projection of `RootState` — only the slices a workbench user
     *  can observe the effects of (elections, electionEvent,
     *  ballotStyles, ballotSelections, castVotes, extra). Booth-only
     *  scratch (auditableBallots, supportMaterials, documents,
     *  confirmationScreenData) is deliberately omitted; see file
     *  header. The field is typed loosely as `Partial<RootState>` so
     *  the serialized form doesn't claim to be a full root — callers
     *  should treat any missing slice as "empty". */
    state: CanonicalRootState
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

/** Slices we persist + compare for dirty detection. Adding a slice
 *  here means "a workbench user can see its effects". Removing one
 *  means "this is booth-internal scratch". The dirty check and the
 *  write path both go through {@link projectCanonicalState} so the
 *  two stay in lockstep. */
export const CANONICAL_STATE_KEYS = [
    "elections",
    "electionEvent",
    "ballotStyles",
    "ballotSelections",
    "castVotes",
    "extra",
] as const

export type CanonicalStateKey = (typeof CANONICAL_STATE_KEYS)[number]
export type CanonicalRootState = Pick<RootState, CanonicalStateKey>

/** `true` for values that carry no scenario information — an empty map
 *  or list. Used by {@link canonicalCompareJson}. */
function isEmptyContainer(v: unknown): boolean {
    if (Array.isArray(v)) return v.length === 0
    if (v && typeof v === "object") return Object.keys(v).length === 0
    return false
}

/**
 * Serialize a canonical projection for **comparison** (not for storage).
 *
 * Identical to `JSON.stringify(projectCanonicalState(...))` except that a
 * slice field which exists on one side, is absent on the other, and holds
 * an *empty* map or list is dropped from both. Such a field carries no
 * scenario information, so its presence must not read as a difference.
 *
 * This exists because portal slices gain fields over time. When upstream
 * added `declinedToVote: {}` to the `extra` slice, every bundled snapshot
 * — all authored before it — instantly compared unequal to the live store
 * and the working copy showed as permanently "modified", with the reload
 * button unable to clear it. Backfilling the snapshots fixes the instance;
 * this fixes the class.
 *
 * Deliberately narrow: only *empty* containers are forgiven. A field
 * holding actual data still counts as a difference on either side, so a
 * real divergence can never be masked.
 */
export function canonicalCompareJson(state: RootState): string {
    const projected = projectCanonicalState(state) as Record<
        string,
        Record<string, unknown>
    >
    const out: Record<string, unknown> = {}
    for (const key of CANONICAL_STATE_KEYS) {
        const slice = projected[key]
        if (!slice || typeof slice !== "object" || Array.isArray(slice)) {
            out[key] = slice
            continue
        }
        const kept: Record<string, unknown> = {}
        for (const field of Object.keys(slice).sort()) {
            if (isEmptyContainer(slice[field])) continue
            kept[field] = slice[field]
        }
        out[key] = kept
    }
    return JSON.stringify(out)
}

/** Reduce a full `RootState` to the canonical scenario projection.
 *  Used everywhere we serialize state (writeSnapshot, saveCheckpoint,
 *  buildCurrentSnapshot) and on the compare side of the dirty check.
 *  Key order is fixed by `CANONICAL_STATE_KEYS` so `JSON.stringify`
 *  comparisons are stable. */
export function projectCanonicalState(state: RootState): CanonicalRootState {
    const out = {} as CanonicalRootState
    for (const k of CANONICAL_STATE_KEYS) {
        // Cast: `out[k]` and `state[k]` have the same per-key type by
        // construction, but TS can't see that through the mapped loop.
        ;(out as Record<string, unknown>)[k] = state[k]
    }
    return out
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
    // Tolerant accessor: returns the slice's values as an array, or
    // `[]` when the slice is missing / null / not an object. Hand-
    // edited or externally-generated snapshots routinely omit slices
    // they don't care about and Object.values(null/undefined) throws,
    // so we normalise here once.
    const sliceValues = <T>(slice: Record<string, T> | null | undefined): T[] =>
        slice != null && typeof slice === "object"
            ? Object.values(slice)
            : []
    const sliceEntries = <T>(
        slice: Record<string, T> | null | undefined
    ): [string, T][] =>
        slice != null && typeof slice === "object"
            ? Object.entries(slice)
            : []
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
                keypair: null,
            }
        )
        for (const election of sliceValues(state?.elections)) {
            if (election) store.dispatch(setElection(election))
        }
        for (const event of sliceValues(state?.electionEvent)) {
            if (event) store.dispatch(setElectionEvent(event))
        }
        for (const ballotStyle of sliceValues(state?.ballotStyles)) {
            if (ballotStyle) store.dispatch(setBallotStyle(ballotStyle))
        }
        for (const [electionId, selection] of sliceEntries(
            state?.ballotSelections
        )) {
            if (!selection) continue
            const ballotStyle = sliceValues(state?.ballotStyles).find(
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
        for (const [, votes] of sliceEntries(state?.castVotes)) {
            if (votes && votes.length > 0) store.dispatch(addCastVotes(votes))
        }
        if (state?.extra) {
            if (typeof state.extra.bypassChooser === "boolean") {
                store.dispatch(setBypassChooser(state.extra.bypassChooser))
            }
            // `isVoted` is a map `{[electionId]: true}`, not a boolean.
            // Reset the slice first so saved-false entries don't get
            // shadowed by leftover live state, then mark each voted
            // election individually (the reducer only sets, never unsets).
            store.dispatch(clearIsVoted())
            const isVoted = state.extra.isVoted
            if (isVoted && typeof isVoted === "object") {
                for (const [electionId, voted] of Object.entries(isVoted)) {
                    if (voted) store.dispatch(setIsVoted(electionId))
                }
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
        state: projectCanonicalState(state),
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
 * Build a {@link PersistedSnapshot} for the live working copy. Same
 * format as {@link writeSnapshot} stores in `localStorage` and the
 * shape that `SnapshotOverviewPage`'s import textarea accepts under
 * the "Import snapshot JSON…" disclosure. Useful for diagnostic
 * pages (e.g. `/diagnostics`) that need to surface the current state in
 * its canonical exchange form.
 */
export function buildCurrentSnapshot(state: RootState): PersistedSnapshot {
    return {
        version: "v1",
        state: projectCanonicalState(state),
        workbench: getWorkbenchState(),
        parentId: currentParentId,
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
    // workbench-owned secret key. The keypair is per-snapshot (one
    // election-wide key shared by every ballot style), matching
    // production trustee-ceremony semantics. Fire-and-forget: failure
    // to decrypt a contest leaves its entry out of `decodedBigInts`,
    // and the tally surfaces that as `no-data`. We deliberately do not
    // await here — the store subscriber is sync, and the cast-vote row
    // is already useful with the plaintext selection alone.
    const keypair = getWorkbenchState().keypair
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
                // keypair was rotated without re-encrypting prior
                // votes).
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
 * Remove any prior cast votes attributed to `voterId` in
 * `electionId`, both from the portal `castVotes` slice and from the
 * workbench overlay (`castBy`, `repairedCastVotes`). Used by the
 * revote/overwrite path: a re-cast from the same voter persona
 * supersedes the previous cast vote rather than stacking on top of
 * it, so the final state always has at most one cast vote per
 * (voter, election).
 *
 * `newCastVoteId` is excluded so this can be safely called *after*
 * the new vote has already landed in the slice (e.g. when discovery
 * is one step behind the dispatch). Cleared overlay rows refer to
 * cast votes that no longer exist in the slice, so dropping them
 * keeps the workbench mini-store consistent with portal state.
 */
function supersedePriorCastVotes(
    store: typeof Store,
    voterId: string,
    electionId: string,
    newCastVoteId: string
): void {
    const wb = getWorkbenchState()
    const electionVotes = store.getState().castVotes[electionId] ?? []
    const priorIds: string[] = []
    for (const cv of electionVotes) {
        if (cv.id === newCastVoteId) continue
        if (wb.castBy[cv.id] === voterId) priorIds.push(cv.id)
    }
    if (priorIds.length === 0) return
    store.dispatch(removeCastVotes(priorIds))
    dropCastVoteOverlay(priorIds)
}

/**
 * Rewrite the portal `ballotStyles` slice from the workbench pool
 * for every election the given voter is eligible for. No-op for
 * voters with no `assignments` entry (legacy snapshots, or imports
 * that didn't model eligibility — those voters see whatever's
 * already in the slice).
 *
 * The dispatched payload is treated opaquely here: the workbench
 * stores pool rows in the same shape the portal's `setBallotStyle`
 * reducer accepts (i.e. the slice's own row shape), so we just hand
 * the row through.
 */
function applyEligibilitySwap(
    store: typeof Store,
    voterId: string
): void {
    const wb = getWorkbenchState()
    const pool = wb.ballotStylePool
    const overrides = getPolicyOverrides()
    // Ephemeral policy overlay (see `policyOverridesStore.ts`):
    // booth open is one of the two boundary points where the
    // operator's per-contest policy overrides are applied. We
    // merge them into the row before dispatch so the portal
    // slice sees the *effective* contest presentation; the
    // baseline pool entry is untouched and the override stays
    // out of persistence.
    if (pool) {
        // Multi-BS snapshot: a pool exists and the active voter's
        // eligibility decides which row goes into the slice. This is
        // also the only branch that performs the eligibility swap;
        // single-BS snapshots already have the correct row loaded.
        for (const electionId of Object.keys(pool)) {
            const row = selectBallotStyleForVoter(voterId, electionId)
            if (!row) continue
            const effective = applyPolicyOverlayToBallotStyleRow(
                row as Parameters<typeof setBallotStyle>[0],
                overrides
            )
            store.dispatch(setBallotStyle(effective))
        }
        return
    }
    // No pool (default/single-BS snapshot): re-dispatch the rows
    // already in the slice so the operator's overlay still reaches
    // the booth. When `overrides` is empty `applyPolicyOverlayToBallotStyleRow`
    // returns the input ref unchanged, so this is a no-op in the
    // common case and doesn't churn the slice.
    const current = store.getState().ballotStyles
    for (const row of Object.values(current)) {
        if (!row) continue
        const effective = applyPolicyOverlayToBallotStyleRow(
            row as Parameters<typeof setBallotStyle>[0],
            overrides
        )
        if (effective !== row) store.dispatch(setBallotStyle(effective))
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
        // workbench overlay. Per new vote:
        //   1. supersede any prior vote by the same voter in the same
        //      election (revote/overwrite — see WORKBENCH.md). The
        //      workbench always allows unlimited overwrites: each new
        //      cast from an attributed voter physically removes the
        //      previous one from the slice + overlay, so the final
        //      state has at most one cast vote per (voter, election)
        //      and the tally counts only the latest input.
        //   2. attributeCastVote(): tag the new vote with the active
        //      voter (no-op when no voter is active — anonymous
        //      casts simply stack, since there's no persona to
        //      overwrite).
        //   3. captureRepairedCastVote(): snapshot the data the demo
        //      path doesn't put on the cast-vote record itself — the
        //      plaintext selection (from state.ballotSelections) and
        //      the encrypted hashable ballot (from
        //      sessionStorage["ballotData"]).
        // After attribution, clear `activeVoterId`: a voter persona
        // casts one ballot per "Vote as" click. Leaving it set would
        // silently attribute the *next* anonymous cast vote to the
        // same persona, which is almost never what the operator
        // wants — the voter detail page re-sets it explicitly on
        // every CTA click.
        const liveState = store.getState()
        for (const votes of Object.values(liveState.castVotes)) {
            if (!votes) continue
            for (const v of votes) {
                if (seenCastVoteIds.has(v.id)) continue
                seenCastVoteIds.add(v.id)
                const activeBefore = getWorkbenchState().activeVoterId
                if (activeBefore && v.election_id) {
                    supersedePriorCastVotes(
                        store,
                        activeBefore,
                        v.election_id,
                        v.id
                    )
                }
                attributeCastVote(v.id)
                tryCaptureRepairedCastVote(liveState, v)
                if (activeBefore) setActiveVoter(null)
            }
        }
        writeSnapshot(store.getState())
    })

    // Workbench-overlay changes (adding a voter, switching active voter)
    // must also flow into the auto-resume slot, otherwise they would be
    // lost on reload. The workbench mini-store fires its listener after
    // every mutation; we just rewrite the snapshot in response.
    //
    // The same subscriber also implements the **eligibility swap**:
    // when `activeVoterId` transitions to a non-null voter that has
    // entries in `workbench.assignments`, we rewrite the portal's
    // `state.ballotStyles[electionId]` from `workbench.ballotStylePool`
    // for every election the voter is eligible for. This keeps the
    // portal-style invariant "one ballot style per (session, election)"
    // intact while letting the workbench hold the full pool of styles
    // out-of-band. See WORKBENCH.md for the rationale.
    let lastActiveVoterId: string | null = getWorkbenchState().activeVoterId
    const unsubWorkbench = subscribeWorkbench(() => {
        const wb = getWorkbenchState()
        if (wb.activeVoterId !== lastActiveVoterId) {
            const prev = lastActiveVoterId
            lastActiveVoterId = wb.activeVoterId
            // Only swap when transitioning *to* a voter — clearing
            // the active voter (e.g. the post-cast retirement above)
            // must leave the slice alone so the booth screen the voter
            // just used can finish rendering.
            if (wb.activeVoterId && wb.activeVoterId !== prev) {
                applyEligibilitySwap(store, wb.activeVoterId)
            }
        }
        if (suspendWrites) return
        writeSnapshot(store.getState())
    })

    // Operator-edited per-contest policy overrides are an ephemeral
    // overlay (see `policyOverridesStore.ts`). When the operator
    // flips an override while a voter is already active (so the
    // `setActiveVoter` transition above won't re-fire), we still
    // want the booth to reflect the change immediately. Re-running
    // the eligibility swap re-dispatches the affected ballot-style
    // row(s) with the new overlay merged in.
    const unsubOverrides = subscribePolicyOverrides(() => {
        const active = getWorkbenchState().activeVoterId
        if (!active) return
        applyEligibilitySwap(store, active)
    })

    return () => {
        unsubStore()
        unsubWorkbench()
        unsubOverrides()
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

/**
 * Replace the live store with a snapshot via a "wipe + reload" rather
 * than an overlay. The snapshot is written straight into the
 * auto-resume slot with the given `parentId`, then the page reloads;
 * the boot path then hydrates a fresh, empty store from that slot, so
 * the resulting state matches the snapshot exactly with no leftovers
 * from before.
 *
 * Use this for Load / Import flows where the operator expects "the
 * working copy now IS this snapshot". Use {@link hydrateFromSnapshot}
 * directly only on the boot path (the store is already empty) or for
 * intentional overlay semantics.
 *
 * `parentId` is what {@link getCurrentParentId} will report after the
 * reload. Pass a tagged id (`bundled:<name>` / `checkpoint:<name>`)
 * for Load, or `null` for Import (the imported state is a root).
 */
export function loadSnapshotViaReload(
    snapshot: PersistedSnapshot,
    parentId: string | null
): void {
    if (typeof localStorage === "undefined") return
    const forBoot: PersistedSnapshot = {
        version: "v1",
        state: snapshot.state,
        workbench: snapshot.workbench,
        parentId,
    }
    localStorage.setItem(PERSISTENCE_KEY, JSON.stringify(forBoot))
    if (typeof window !== "undefined") window.location.reload()
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
        state: projectCanonicalState(store.getState()),
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
    // Bump the workbench store so any subscribers (notably the
    // inspector rail's `useCheckpointList` and `useCurrentParentId`)
    // re-read. Saving doesn't actually mutate the overlay, but the
    // checkpoint index and the current parent id both changed and
    // we want the rail to reflect that immediately.
    replaceWorkbenchState(getWorkbenchState())
    return meta
}

/**
 * Read a saved checkpoint's full snapshot (state + workbench overlay +
 * parentId) without hydrating it into the store. Returns `null` if no
 * such checkpoint exists or the stored payload is unparseable.
 */
export function readCheckpointSnapshot(rawName: string): PersistedSnapshot | null {
    const name = normalizeCheckpointName(rawName)
    if (typeof localStorage === "undefined") return null
    const raw = localStorage.getItem(CHECKPOINT_PREFIX + name)
    if (!raw) return null
    try {
        const snapshot = JSON.parse(raw) as PersistedSnapshot
        if (snapshot.version !== "v1") return null
        return snapshot
    } catch (e) {
        console.warn(
            `[workbench/persistence] failed to parse checkpoint "${name}":`,
            e
        )
        return null
    }
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
    // Bump subscribers so the inspector rail drops the row.
    replaceWorkbenchState(getWorkbenchState())
}

/**
 * Persist a snapshot under a fresh checkpoint name *without* hydrating
 * it into the live store. Used by the raw-JSON import flows
 * (snapshot / ballot-style / velvet) to give the imported state an
 * identity in the checkpoint index *before* the page reloads — that
 * way the post-reload boot lands on a snapshot whose `parentId` points
 * back at this freshly-materialized checkpoint, and the rail can
 * highlight it as the active snapshot.
 *
 * Distinct from {@link saveCheckpoint}, which captures *the live
 * store's* current state and operates after a Save action. This
 * helper instead writes a *given* snapshot to localStorage and the
 * index, and is meant to be paired with
 * {@link loadSnapshotViaReload}` so the just-materialized
 * checkpoint becomes the active snapshot after reload.
 *
 * Returns the tagged checkpoint id (`checkpoint:<name>`), suitable
 * for passing straight to {@link loadSnapshotViaReload} as the new
 * working copy's `parentId`.
 */
export function materializeAsCheckpoint(
    snapshot: PersistedSnapshot,
    rawName: string
): string {
    const name = normalizeCheckpointName(rawName)
    if (typeof localStorage === "undefined") {
        throw new Error(
            "Cannot materialize checkpoint: localStorage is unavailable."
        )
    }
    // Persist the snapshot in the exact shape `loadCheckpoint` /
    // `readCheckpointSnapshot` expect — same as `saveCheckpoint`
    // writes — so subsequent Load / Inspect flows treat it
    // indistinguishably from an operator-saved checkpoint.
    const blob: PersistedSnapshot = {
        version: "v1",
        state: snapshot.state,
        workbench: snapshot.workbench,
        // An imported snapshot is conceptually a root in *this*
        // workbench's lineage even if its source JSON happened to
        // carry a parentId from some other workbench. We deliberately
        // discard the foreign parentId here: the rail would otherwise
        // render this checkpoint as an orphan pointing at a snapshot
        // we know nothing about.
        parentId: null,
    }
    localStorage.setItem(CHECKPOINT_PREFIX + name, JSON.stringify(blob))
    const meta: CheckpointMeta = {
        name,
        savedAt: new Date().toISOString(),
        parentId: null,
    }
    const next = readCheckpointIndex().filter((e) => e.name !== name)
    next.push(meta)
    writeCheckpointIndex(next)
    return checkpointId(name)
}

/**
 * Resolve a tagged snapshot id (`bundled:<name>` /
 * `checkpoint:<name>`) to the underlying {@link PersistedSnapshot},
 * or `null` if no such snapshot exists or the id is malformed.
 *
 * Used by the dirty-check infrastructure: comparing the live store
 * against the *currently active* snapshot requires loading whatever
 * snapshot `getCurrentParentId()` points at, regardless of whether
 * it is a bundled fixture or a saved checkpoint.
 */
export function loadSnapshotById(id: string | null): PersistedSnapshot | null {
    if (id == null) return null
    if (id.startsWith("bundled:")) {
        return loadBundledSnapshot(id.slice("bundled:".length))
    }
    if (id.startsWith("checkpoint:")) {
        return readCheckpointSnapshot(id.slice("checkpoint:".length))
    }
    return null
}
