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
}

/** Per-cast-vote bridge record. See {@link WorkbenchExtraState.repairedCastVotes}. */
export interface RepairedCastVote {
    /** Real election id, taken from `ballotStyle.election_id`. NOT the
     *  `eventId` that `useAddFakeCastVote` writes into the cast-vote
     *  record. */
    electionId: string
    /** The ballot style that was active when the vote was cast. */
    ballotStyleId: string
    /** Plaintext selection snapshot taken from `state.ballotSelections`
     *  at cast time. Stored as JSON-safe `unknown` so this file does
     *  not have to import portal types; consumers cast to
     *  `BallotSelection` from `@sequentech/ui-core` when they need
     *  structured access. */
    selection: unknown
    /** `sessionStorage["ballotData"]["ballot"]` at cast time, if
     *  available. Stringified `IHashableSingleBallot` (encrypted). The
     *  workbench keeps it for display / forensic purposes only — it
     *  has no decryption keys, so this string can NOT be tallied. */
    hashableBallotJson: string | null
    /** ISO-8601 timestamp of capture. */
    capturedAt: string
}

const EMPTY_STATE: WorkbenchExtraState = Object.freeze({
    voters: [],
    activeVoterId: null,
    castBy: {},
    repairedCastVotes: {},
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

/** Seed an empty workbench with two demo voters so the directory page
 *  is not empty on first boot. Idempotent: does nothing if there are
 *  already voters in the directory. */
export function seedDemoVoters(): void {
    if (state.voters.length > 0) return
    setState({
        ...state,
        voters: sortVoters([
            {id: generateVoterId(), displayName: "Alice"},
            {id: generateVoterId(), displayName: "Bob"},
        ]),
    })
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
                repairedCastVotes[k] = v as RepairedCastVote
            }
        }
    }
    return {voters: sortVoters(voters), activeVoterId, castBy, repairedCastVotes}
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
