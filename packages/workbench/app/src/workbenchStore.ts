// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Workbench-only state, kept entirely outside the voting-portal Redux store.
//
// Why a separate store rather than another portal slice:
//   - We MUST NOT modify `voting-portal/src/store/store.ts` (see
//     LIFTING.md section I). Adding a slice to the production store is
//     a portal-source change.
//   - The state we need is operator-facing scenario data (voter
//     directory, active impersonation, cast-vote attribution ledger),
//     not anything the booth itself reads. Bolting it onto the portal
//     store would muddy the line between "production state" (lifted
//     verbatim) and "workbench overlay" (workbench-owned).
//
// Implementation is the tiniest useful publish/subscribe store + React
// integration via `useSyncExternalStore`. No external dependency, ~50
// lines.

import {useSyncExternalStore} from "react"

/** A voter persona in the workbench directory. */
export interface Voter {
    /** Workbench-generated identifier (not a real DB id). Stable across
     *  reloads. */
    id: string
    /** Free-text label shown in the UI. */
    displayName: string
    /** Optional operator notes. Kept short. */
    notes?: string
}

export interface WorkbenchExtraState {
    /** Voter directory, kept sorted by displayName for stable UI order. */
    voters: Voter[]
    /** ID of the voter currently being impersonated, or `null` for an
     *  anonymous booth session (the default). When a cast vote is
     *  observed, it is attributed to this voter — see
     *  {@link attributeCastVote}. */
    activeVoterId: string | null
    /** Map from cast-vote id (as produced by the portal's `addCastVotes`
     *  dispatch) to the voter who was active at the time. This is the
     *  workbench's substitute for the production `voter_id_string`
     *  field, which `useAddFakeCastVote` always sets to `null` in
     *  DISABLE_AUTH mode. */
    castBy: Record<string, string>
    /** Reconciled per-cast-vote bridge data. The portal's
     *  `useAddFakeCastVote` writes `election_id = eventId` and
     *  `content = ""` under DISABLE_AUTH, so the cast-vote record in
     *  Redux is not faithful to what was actually voted. This map fills
     *  in the missing pieces from sources outside that record: the real
     *  election id (taken from the matching ballot style), the plaintext
     *  selection (snapshotted from `state.ballotSelections` at cast
     *  time), and the encrypted hashable-ballot JSON (taken from
     *  `sessionStorage["ballotData"]`, opaque to the workbench because
     *  we lack the decryption keys). */
    repairedCastVotes: Record<string, RepairedCastVote>
    /** Workbench-owned ElGamal keypairs, keyed by ballot-style id. A
     *  ballot style's `public_key` (in Redux) is paired with the matching
     *  secret key held here, so the encrypt path uses our pk and the
     *  decrypt bridge can recover plaintexts under the same scope. The
     *  key is per-ballot-style because that is the field name production
     *  uses for the encryption key; multiple ballot styles in one scenario
     *  can each carry their own pair. Bundled snapshots ship both halves;
     *  the loader rejects snapshots whose ballot styles lack a matching
     *  entry here. Production has no analogue: real election keys are
     *  threshold-shared between trustees and never live as a single
     *  secret anywhere. */
    keypairs: Record<string, WorkbenchKeypair>
    /** Workbench-only pool of *all* ballot styles available per
     *  election, keyed by `election_id`. The portal's `ballotStyles`
     *  slice only ever holds one ballot style per election at a time
     *  (the one the current session is eligible for), so we keep the
     *  full set out-of-band here. The active-voter swap (see
     *  {@link setActiveVoter} listener in `persistence.ts`) rewrites
     *  the slice from this pool according to {@link assignments}.
     *
     *  Each row is the same shape as the portal `ballotStyles` slice
     *  row (kept as `unknown` here so this file does not import
     *  voting-portal types). Snapshots written before this overlay
     *  existed simply omit it; in that case the active-voter swap is
     *  a no-op and the slice retains whatever was hydrated. */
    ballotStylePool?: Record<string, unknown[]>
    /** Per-voter eligibility map: which ballot styles each voter may
     *  receive, by ballot-style id. The active-voter swap intersects
     *  `assignments[voterId]` with the per-election entries of
     *  {@link ballotStylePool} to pick which BS to dispatch into the
     *  Redux slice.
     *
     *  Optional: snapshots without `assignments` leave the slice
     *  untouched on voter change, matching pre-eligibility
     *  behaviour. */
    assignments?: Record<string, string[]>    /** Per-contest cache of the most recent manual tally run, keyed
     *  by contestId. Persisted across navigation and reloads so the
     *  "results are stale" indicator on `ContestDetailPage` can
     *  fire even when the operator leaves the page to cast a ballot
     *  (which is the only realistic way to change tally inputs in a
     *  single-tab workbench session).
     *
     *  The cached `outcome` is fully JSON-serialisable (a parsed
     *  velvet-wasm `ContestResult`), so it round-trips via
     *  `PersistedSnapshot` without special-casing. */
    tallyRuns?: Record<string, TallyRun>
}

/** One entry in {@link WorkbenchExtraState.tallyRuns}. `fingerprint`
 *  is the inputs hash the run was computed against — a stale
 *  indicator fires when the current decodedRows hash differs from
 *  this value. `outcome` is `null` when the run failed; `errorMessage`
 *  is set in that case. */
export interface TallyRun {
    fingerprint: string
    outcome: import("./electionTally").ContestTallyOutcome | null
    errorMessage: string | null
    /** ISO-8601 timestamp of when the operator pressed Run tally. */
    ranAt: string}

/** Ristretto ElGamal keypair owned by the workbench. Both halves are
 *  base64-no-pad strings as produced by `velvet-wasm::generate_keypair`
 *  (strand/borsh-serialised). */
export interface WorkbenchKeypair {
    pkB64: string
    skB64: string
}

/** Per-cast-vote bridge record. See {@link WorkbenchExtraState.repairedCastVotes}. */
export interface RepairedCastVote {
    /** Real election id, taken from `ballotStyle.election_id`. */
    electionId: string
    /** The ballot style that was active when the vote was cast. */
    ballotStyleId: string
    /** Plaintext selection snapshot taken from `state.ballotSelections`
     *  at cast time. Stored as JSON-safe `unknown` so this file does
     *  not have to import portal types; consumers cast to
     *  `BallotSelection` from `@sequentech/ui-core` when they need
     *  structured access. */
    selection: unknown
    /** Per-contest decimal `BigUint` strings recovered by decrypting
     *  `castVote.content` with the workbench-owned secret key. Keyed
     *  by `contestId`. Empty when decryption could not run (e.g.
     *  hydrated from a pre-step-6 snapshot, missing keypair, malformed
     *  ciphertext). Same bytes `encodeBallot` would produce from the
     *  matching selection, which is what the round-trip badge checks.
     */
    decodedBigInts: Record<string, string>
    /** ISO-8601 timestamp of capture. */
    capturedAt: string
}

const EMPTY_STATE: WorkbenchExtraState = Object.freeze({
    voters: [],
    activeVoterId: null,
    castBy: {},
    repairedCastVotes: {},
    keypairs: {},
})

let state: WorkbenchExtraState = EMPTY_STATE
const listeners = new Set<() => void>()

/** Return the current workbench-extra state. Identity-stable until the
 *  next mutation, which is what `useSyncExternalStore` requires. */
export function getWorkbenchState(): WorkbenchExtraState {
    return state
}

/** Subscribe to workbench-state changes. Returns an unsubscribe fn. */
export function subscribeWorkbench(listener: () => void): () => void {
    listeners.add(listener)
    return () => listeners.delete(listener)
}

/** React hook over the workbench mini-store. Selector projects a slice
 *  of state; component re-renders when the selector's output changes
 *  (by referential equality). */
export function useWorkbench<T>(
    selector: (s: WorkbenchExtraState) => T
): T {
    return useSyncExternalStore(
        subscribeWorkbench,
        () => selector(state),
        () => selector(EMPTY_STATE)
    )
}

function setState(next: WorkbenchExtraState): void {
    state = next
    listeners.forEach((l) => l())
}

// ---------------------------------------------------------------------------
// Mutations
// ---------------------------------------------------------------------------

/** Add a new voter to the directory. Returns the created voter (with
 *  its generated id). Names are trimmed; empty names are rejected so
 *  the UI can surface a precise error. */
export function addVoter(rawDisplayName: string, notes?: string): Voter {
    const displayName = rawDisplayName.trim()
    if (displayName.length === 0) {
        throw new Error("Voter displayName must not be empty.")
    }
    const voter: Voter = {
        id: generateVoterId(),
        displayName,
        ...(notes && notes.trim().length > 0 ? {notes: notes.trim()} : {}),
    }
    setState({
        ...state,
        voters: sortVoters([...state.voters, voter]),
    })
    return voter
}

export function removeVoter(voterId: string): void {
    // Cascade: drop from active, drop attribution ledger entries.
    // Attribution rows for the deleted voter become "(deleted)" in the
    // UI rather than vanishing silently, so we keep the keys but point
    // them at a sentinel. Simpler: just delete them — the cast-vote
    // record itself isn't lost (it lives in the portal Redux store),
    // only the attribution. Operator can re-impersonate and re-cast if
    // they care.
    const nextCastBy: Record<string, string> = {}
    for (const [voteId, vId] of Object.entries(state.castBy)) {
        if (vId !== voterId) nextCastBy[voteId] = vId
    }
    setState({
        voters: state.voters.filter((v) => v.id !== voterId),
        activeVoterId:
            state.activeVoterId === voterId ? null : state.activeVoterId,
        castBy: nextCastBy,
    })
}

export function setActiveVoter(voterId: string | null): void {
    if (voterId === state.activeVoterId) return
    if (voterId !== null && !state.voters.some((v) => v.id === voterId)) {
        throw new Error(`Unknown voter id: ${voterId}`)
    }
    setState({...state, activeVoterId: voterId})
}

/** Attribute a freshly-observed cast vote to the currently active
 *  voter, if one is set and the vote isn't already attributed. No-op
 *  otherwise. Called from the cast-votes-watcher subscription. */
export function attributeCastVote(castVoteId: string): void {
    if (!state.activeVoterId) return
    if (state.castBy[castVoteId]) return
    setState({
        ...state,
        castBy: {...state.castBy, [castVoteId]: state.activeVoterId},
    })
}

/** Snapshot the bridge data for a freshly-observed cast vote. No-op if
 *  the vote is already in the map — the first observation wins, so
 *  replays during hydration do not clobber recorded data. */
export function captureRepairedCastVote(
    castVoteId: string,
    repaired: RepairedCastVote
): void {
    if (state.repairedCastVotes[castVoteId]) return
    setState({
        ...state,
        repairedCastVotes: {
            ...state.repairedCastVotes,
            [castVoteId]: repaired,
        },
    })
}

/** Merge per-contest decoded `BigUint` strings into an already-captured
 *  cast vote. Used by the async decrypt path in the bridge: the
 *  synchronous capture writes the plaintext selection immediately so
 *  the UI updates, and this merge fills in `decodedBigInts` once the
 *  wasm-side decrypt+decode resolves. No-op if the cast vote has not
 *  been captured yet (defensive — should not happen because the bridge
 *  always calls `captureRepairedCastVote` first). */
export function setRepairedDecodedBigInts(
    castVoteId: string,
    decodedBigInts: Record<string, string>
): void {
    const existing = state.repairedCastVotes[castVoteId]
    if (!existing) return
    setState({
        ...state,
        repairedCastVotes: {
            ...state.repairedCastVotes,
            [castVoteId]: {
                ...existing,
                decodedBigInts: {...existing.decodedBigInts, ...decodedBigInts},
            },
        },
    })
}

/** Drop the workbench bridge data and voter attribution for a set of
 *  cast-vote ids. Called when those cast votes have been removed from
 *  the portal `castVotes` slice (revote/overwrite path) so we do not
 *  carry stale entries pointing at ids that no longer exist. */
export function dropCastVoteOverlay(castVoteIds: string[]): void {
    if (castVoteIds.length === 0) return
    const drop = new Set(castVoteIds)
    const nextCastBy: Record<string, string> = {}
    for (const [k, v] of Object.entries(state.castBy)) {
        if (!drop.has(k)) nextCastBy[k] = v
    }
    const nextRepaired: Record<string, RepairedCastVote> = {}
    for (const [k, v] of Object.entries(state.repairedCastVotes)) {
        if (!drop.has(k)) nextRepaired[k] = v
    }
    setState({
        ...state,
        castBy: nextCastBy,
        repairedCastVotes: nextRepaired,
    })
}

/** Install a keypair for a ballot style. First call per id wins;
 *  subsequent calls are no-ops so a stray re-seed cannot invalidate an
 *  already-captured cast vote (which was encrypted under the existing
 *  pk for that ballot style). Operators who want a fresh keypair edit
 *  the snapshot directly; see LIFTING.md section M. */
export function setKeypair(ballotStyleId: string, kp: WorkbenchKeypair): void {
    if (state.keypairs[ballotStyleId]) return
    setState({
        ...state,
        keypairs: {...state.keypairs, [ballotStyleId]: kp},
    })
}

/** Record the result of a manual tally run for a contest. Replaces
 *  any previous entry under the same `contestId`. */
export function recordTallyRun(contestId: string, run: TallyRun): void {
    setState({
        ...state,
        tallyRuns: {...(state.tallyRuns ?? {}), [contestId]: run},
    })
}

/** For the given voter and election, return the ballot-style row from
 *  {@link WorkbenchExtraState.ballotStylePool} that the voter is
 *  assigned to. Returns `null` when:
 *
 *    - the pool has no entries for that election,
 *    - the voter has no `assignments` entry (legacy snapshot or
 *      single-voter import where every voter sees every BS), or
 *    - no row in the pool matches one of the voter's assigned ids.
 *
 *  Used by the persistence-layer subscriber that rewrites the portal
 *  `ballotStyles` slice on every `setActiveVoter` transition. */
export function selectBallotStyleForVoter(
    voterId: string,
    electionId: string
): unknown | null {
    const pool = state.ballotStylePool?.[electionId]
    if (!pool || pool.length === 0) return null
    const assigned = state.assignments?.[voterId]
    if (!assigned || assigned.length === 0) return null
    for (const row of pool) {
        const id = (row as {id?: unknown}).id
        if (typeof id === "string" && assigned.includes(id)) return row
    }
    return null
}

// ---------------------------------------------------------------------------
// Persistence integration
// ---------------------------------------------------------------------------

/** Replace the current workbench state wholesale. Used by the
 *  persistence layer when hydrating from a snapshot. Skips notifying
 *  listeners until after the assignment so that hydration is a single
 *  React batch. */
export function replaceWorkbenchState(next: WorkbenchExtraState): void {
    setState(normalizeIncoming(next))
}

function normalizeIncoming(incoming: WorkbenchExtraState): WorkbenchExtraState {
    // Be defensive against snapshots from older versions or hand-edits.
    const voters = Array.isArray(incoming.voters)
        ? incoming.voters.filter(
              (v): v is Voter =>
                  !!v &&
                  typeof v.id === "string" &&
                  typeof v.displayName === "string"
          )
        : []
    const ids = new Set(voters.map((v) => v.id))
    const activeVoterId =
        typeof incoming.activeVoterId === "string" &&
        ids.has(incoming.activeVoterId)
            ? incoming.activeVoterId
            : null
    const castBy: Record<string, string> = {}
    if (incoming.castBy && typeof incoming.castBy === "object") {
        for (const [k, v] of Object.entries(incoming.castBy)) {
            if (typeof k === "string" && typeof v === "string" && ids.has(v)) {
                castBy[k] = v
            }
        }
    }
    const repairedCastVotes: Record<string, RepairedCastVote> = {}
    if (
        incoming.repairedCastVotes &&
        typeof incoming.repairedCastVotes === "object"
    ) {
        for (const [k, v] of Object.entries(incoming.repairedCastVotes)) {
            if (
                typeof k === "string" &&
                v &&
                typeof v === "object" &&
                typeof (v as RepairedCastVote).electionId === "string" &&
                typeof (v as RepairedCastVote).ballotStyleId === "string"
            ) {
                // Back-compat: snapshots written before `decodedBigInts`
                // existed simply rehydrate with an empty map. The bridge
                // will not back-fill on hydrate (the source ciphertexts
                // are still on the cast-vote record, but re-running
                // decrypt on hydrate would risk doing so under a
                // different keypair if the user has reset workbench
                // state in between).
                const raw = v as RepairedCastVote & {
                    decodedBigInts?: unknown
                }
                const decoded: Record<string, string> = {}
                if (
                    raw.decodedBigInts &&
                    typeof raw.decodedBigInts === "object"
                ) {
                    for (const [cid, big] of Object.entries(
                        raw.decodedBigInts as Record<string, unknown>
                    )) {
                        if (typeof big === "string") decoded[cid] = big
                    }
                }
                repairedCastVotes[k] = {
                    electionId: raw.electionId,
                    ballotStyleId: raw.ballotStyleId,
                    selection: raw.selection,
                    decodedBigInts: decoded,
                    capturedAt: raw.capturedAt,
                }
            }
        }
    }
    const keypairs: Record<string, WorkbenchKeypair> = {}
    const incomingKeypairs = (
        incoming as WorkbenchExtraState & {keypairs?: unknown}
    ).keypairs
    if (incomingKeypairs && typeof incomingKeypairs === "object") {
        for (const [bsId, kp] of Object.entries(
            incomingKeypairs as Record<string, unknown>
        )) {
            if (
                typeof bsId === "string" &&
                kp &&
                typeof kp === "object" &&
                typeof (kp as WorkbenchKeypair).pkB64 === "string" &&
                typeof (kp as WorkbenchKeypair).skB64 === "string"
            ) {
                keypairs[bsId] = {
                    pkB64: (kp as WorkbenchKeypair).pkB64,
                    skB64: (kp as WorkbenchKeypair).skB64,
                }
            }
        }
    }
    // Eligibility overlay: ballotStylePool and assignments are both
    // optional. They're round-tripped opaquely (the workbench does not
    // peer into pool rows; the persistence layer interprets them via
    // setBallotStyle dispatches when the active voter changes). Bad
    // input shapes are silently dropped so legacy snapshots and
    // hand-edits don't crash hydration.
    let ballotStylePool: Record<string, unknown[]> | undefined
    const incomingPool = (
        incoming as WorkbenchExtraState & {ballotStylePool?: unknown}
    ).ballotStylePool
    if (incomingPool && typeof incomingPool === "object") {
        ballotStylePool = {}
        for (const [electionId, rows] of Object.entries(
            incomingPool as Record<string, unknown>
        )) {
            if (typeof electionId === "string" && Array.isArray(rows)) {
                ballotStylePool[electionId] = rows.filter(
                    (r) => r != null && typeof r === "object"
                )
            }
        }
    }
    let assignments: Record<string, string[]> | undefined
    const incomingAssignments = (
        incoming as WorkbenchExtraState & {assignments?: unknown}
    ).assignments
    if (incomingAssignments && typeof incomingAssignments === "object") {
        assignments = {}
        for (const [voterId, bsIds] of Object.entries(
            incomingAssignments as Record<string, unknown>
        )) {
            if (
                typeof voterId === "string" &&
                ids.has(voterId) &&
                Array.isArray(bsIds)
            ) {
                assignments[voterId] = bsIds.filter(
                    (x): x is string => typeof x === "string"
                )
            }
        }
    }
    // Cached tally runs: round-trip the parsed `ContestResult` blob
    // opaquely (it's velvet-wasm output, we don't peer into it).
    // Reject entries whose shape doesn't look right so legacy/
    // hand-edited snapshots don't crash hydration.
    let tallyRuns: Record<string, TallyRun> | undefined
    const incomingTallyRuns = (
        incoming as WorkbenchExtraState & {tallyRuns?: unknown}
    ).tallyRuns
    if (incomingTallyRuns && typeof incomingTallyRuns === "object") {
        tallyRuns = {}
        for (const [contestId, run] of Object.entries(
            incomingTallyRuns as Record<string, unknown>
        )) {
            if (
                typeof contestId === "string" &&
                run &&
                typeof run === "object" &&
                typeof (run as TallyRun).fingerprint === "string" &&
                typeof (run as TallyRun).ranAt === "string"
            ) {
                const r = run as TallyRun
                tallyRuns[contestId] = {
                    fingerprint: r.fingerprint,
                    outcome:
                        r.outcome && typeof r.outcome === "object"
                            ? r.outcome
                            : null,
                    errorMessage:
                        typeof r.errorMessage === "string"
                            ? r.errorMessage
                            : null,
                    ranAt: r.ranAt,
                }
            }
        }
    }
    return {
        voters: sortVoters(voters),
        activeVoterId,
        castBy,
        repairedCastVotes,
        keypairs,
        ...(ballotStylePool ? {ballotStylePool} : {}),
        ...(assignments ? {assignments} : {}),
        ...(tallyRuns ? {tallyRuns} : {}),
    }
}

function sortVoters(vs: Voter[]): Voter[] {
    return [...vs].sort((a, b) =>
        a.displayName.localeCompare(b.displayName, undefined, {
            sensitivity: "base",
        })
    )
}

function generateVoterId(): string {
    // crypto.randomUUID is available in all modern browsers we target.
    // Falls back to a timestamp-based id only if the host is so
    // ancient it lacks it (workbench is dev-only, this branch is
    // really just belt and braces).
    if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
        return crypto.randomUUID()
    }
    return `voter-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}
