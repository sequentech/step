// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Ephemeral per-contest policy override overlay for the workbench.
//
// The six vote-validation policies catalogued in
// `packages/workbench/docs/FIXTURE_VARIANCE.md` §10.A —
// `invalid_vote_policy`, `over_vote_policy`, `under_vote_policy`,
// `blank_vote_policy`, plus the preferential-only
// `duplicated_rank_policy` and `preference_gaps_policy` — are the
// fields a workbench operator most wants to flip on the fly while
// exercising the booth and the tally sandbox. The snapshot's
// canonical `Contest.presentation` carries the *baseline* values; this
// store carries an *override layer* on top, keyed by `contest_id`,
// that is applied at exactly two boundary points:
//
//   1. Booth open — when the persistence layer's `setActiveVoter`
//      listener dispatches `setBallotStyle(row)` for the voter the
//      operator just clicked "Cast vote" as. See
//      `applyEligibilitySwap` in persistence.ts.
//   2. Tally run — when `TallyPage.handleRunTally` reads the contest
//      JSON out of its editable textarea and hands it to velvet-wasm.
//
// Direction of merge: overrides win for the six policy fields,
// baseline wins for everything else. The textarea on the tally page
// stays the authoritative source for non-policy edits (candidates,
// max_votes, names, ...); the policy panel is the structured source
// for the six policies and re-asserts itself at run time.
//
// Why a separate module-level store rather than another field on
// `WorkbenchExtraState`:
//
//   - The overlay is intentionally **ephemeral**. Persisting it
//     would make snapshot diffs lie ("this scenario behaves the
//     same as before" while the operator silently has live
//     overrides). Keeping it in its own module means the
//     persistence layer cannot accidentally pick it up.
//   - It is also intentionally per-tab / per-session: opening a
//     second workbench tab starts with a clean overlay. That falls
//     out naturally from not touching localStorage.
//
// Implementation mirrors `workbenchStore.ts`: a tiny publish/subscribe
// signal driven by `useSyncExternalStore`. ~50 lines.

import {useSyncExternalStore} from "react"

import type {
    EBlankVotePolicy,
    EDuplicatedRankPolicy,
    EInvalidVotePolicy,
    EOverVotePolicy,
    EPreferenceGapsPolicy,
    EUnderVotePolicy,
} from "@sequentech/ui-core"

/** The fields that may be overridden. All optional: a missing field
 *  means "use the contest's baseline value".
 *
 *  Two flavours coexist here:
 *
 *    - **Policy keys** (`POLICY_KEYS`): the six vote-validation
 *      policies that live on `Contest.presentation`. Overlay merges
 *      into the presentation object.
 *    - **Bounds keys** (`BOUNDS_KEYS`): `min_votes` / `max_votes`.
 *      They aren't policies — they're the *frame* (the valid range)
 *      that makes most of the policies reachable in the first place
 *      (e.g. `blank_vote_policy` is inert unless `min_votes == 0`;
 *      `under_vote_policy` is inert unless `max_votes - min_votes
 *      >= 2`). Without exposing them, flipping a policy often looks
 *      like a no-op. Overlay applies them at the contest level, not
 *      inside `presentation`. */
export interface ContestPolicyOverlay {
    invalid_vote_policy?: EInvalidVotePolicy
    over_vote_policy?: EOverVotePolicy
    under_vote_policy?: EUnderVotePolicy
    blank_vote_policy?: EBlankVotePolicy
    /** Preferential contests only (InstantRunoff, Borda*). Ignored
     *  for Plurality. */
    duplicated_rank_policy?: EDuplicatedRankPolicy
    /** Preferential contests only. */
    preference_gaps_policy?: EPreferenceGapsPolicy
    /** Contest-level bound. Non-negative integer. */
    min_votes?: number
    /** Contest-level bound. Non-negative integer; should be >=
     *  `min_votes`. */
    max_votes?: number
}

/** Union of the keys above. Useful for typed control wiring. */
export type ContestPolicyKey = keyof ContestPolicyOverlay

/** The six presentation-level policy keys, in display order. */
export const POLICY_KEYS: ReadonlyArray<ContestPolicyKey> = [
    "invalid_vote_policy",
    "over_vote_policy",
    "under_vote_policy",
    "blank_vote_policy",
    "duplicated_rank_policy",
    "preference_gaps_policy",
]

/** The two contest-level bounds. They live on the contest object
 *  itself, not on `presentation`, so the apply helpers splice them
 *  separately from `POLICY_KEYS`. */
export const BOUNDS_KEYS: ReadonlyArray<ContestPolicyKey> = [
    "min_votes",
    "max_votes",
]

/** Preferential-only subset — used to gate UI on plurality contests. */
export const PREFERENTIAL_ONLY_KEYS: ReadonlySet<ContestPolicyKey> = new Set([
    "duplicated_rank_policy",
    "preference_gaps_policy",
])

type OverridesMap = Readonly<Record<string, ContestPolicyOverlay>>

const EMPTY_MAP: OverridesMap = Object.freeze({})
const EMPTY_OVERLAY: ContestPolicyOverlay = Object.freeze({})

let state: OverridesMap = EMPTY_MAP
const listeners = new Set<() => void>()

function notify(): void {
    listeners.forEach((l) => l())
}

function subscribe(listener: () => void): () => void {
    listeners.add(listener)
    return () => listeners.delete(listener)
}

/** Subscribe to override-map changes from non-React call sites
 *  (e.g. the persistence layer's eligibility-swap listener, which
 *  re-applies the overlay when the operator edits an override while a
 *  voter is already active). Returns an unsubscribe function. */
export function subscribePolicyOverrides(
    listener: () => void
): () => void {
    return subscribe(listener)
}

/** Return the full override map. Identity-stable until the next
 *  mutation. */
export function getPolicyOverrides(): OverridesMap {
    return state
}

/** Return the overlay for a single contest. Identity-stable; returns
 *  a shared empty object when no overrides exist for the id. */
export function getContestPolicyOverlay(
    contestId: string
): ContestPolicyOverlay {
    return state[contestId] ?? EMPTY_OVERLAY
}

/** React hook over the overrides store. The selector pattern matches
 *  {@link useWorkbench} so call sites read identically. */
export function usePolicyOverrides<T>(
    selector: (overrides: OverridesMap) => T
): T {
    return useSyncExternalStore(
        subscribe,
        () => selector(state),
        () => selector(EMPTY_MAP)
    )
}

/** Convenience hook: per-contest overlay (identity-stable). */
export function useContestPolicyOverlay(
    contestId: string
): ContestPolicyOverlay {
    return usePolicyOverrides((m) => m[contestId] ?? EMPTY_OVERLAY)
}

/** Set or clear a single policy on a single contest. Passing
 *  `undefined` reverts the field to the contest's baseline. When all
 *  six fields are clear, the per-contest entry is dropped from the
 *  map so `JSON.stringify(getPolicyOverrides())` is empty in the
 *  no-override state (useful for snapshot/diff diagnostics, even
 *  though we don't persist this store). */
export function setPolicyOverride<K extends ContestPolicyKey>(
    contestId: string,
    field: K,
    value: ContestPolicyOverlay[K] | undefined
): void {
    const prev = state[contestId] ?? EMPTY_OVERLAY
    const next: ContestPolicyOverlay = {...prev}
    if (value === undefined) {
        delete next[field]
    } else {
        next[field] = value
    }
    const nextMap: Record<string, ContestPolicyOverlay> = {...state}
    if (Object.keys(next).length === 0) {
        delete nextMap[contestId]
    } else {
        nextMap[contestId] = next
    }
    state = nextMap
    notify()
}

/** Drop every override for a single contest. */
export function clearContestOverrides(contestId: string): void {
    if (!(contestId in state)) return
    const nextMap: Record<string, ContestPolicyOverlay> = {...state}
    delete nextMap[contestId]
    state = nextMap
    notify()
}

/** Drop every override across every contest. */
export function clearAllOverrides(): void {
    if (Object.keys(state).length === 0) return
    state = EMPTY_MAP
    notify()
}

// ---------------------------------------------------------------------------
// Pure overlay-apply helpers
// ---------------------------------------------------------------------------

/** Minimal shape we need from a contest descriptor: an id and an
 *  optional `presentation` object. Workbench code keeps contests
 *  largely opaque (`Record<string, unknown>`), so we mirror that
 *  here. */
interface ContestLike {
    id?: unknown
    presentation?: Record<string, unknown> | null | undefined
    [key: string]: unknown
}

/** Merge `overlay` into `contest.presentation`. Pure: returns a
 *  shallow-cloned contest with a shallow-cloned presentation object;
 *  the input is not mutated. When the overlay is empty, returns the
 *  original contest unchanged (referentially) so callers can use
 *  `===` to detect "no work to do". */
export function applyPolicyOverlayToContest<T extends ContestLike>(
    contest: T,
    overlay: ContestPolicyOverlay | undefined
): T {
    if (!overlay) return contest
    // Presentation-level merge for the six policies.
    const mergedPolicies: Record<string, unknown> = {}
    let anyPolicy = false
    for (const k of POLICY_KEYS) {
        const v = overlay[k]
        if (v !== undefined) {
            mergedPolicies[k] = v
            anyPolicy = true
        }
    }
    // Contest-level splice for min_votes / max_votes. These don't
    // belong inside `presentation`; the booth and the tally read them
    // from the contest object directly (see
    // `check_min_vote_policy`, `check_over_vote_policy`).
    const mergedBounds: Record<string, unknown> = {}
    let anyBound = false
    for (const k of BOUNDS_KEYS) {
        const v = overlay[k]
        if (v !== undefined) {
            mergedBounds[k] = v
            anyBound = true
        }
    }
    if (!anyPolicy && !anyBound) return contest
    const next: ContestLike = {...contest, ...mergedBounds}
    if (anyPolicy) {
        next.presentation = {
            ...(contest.presentation ?? {}),
            ...mergedPolicies,
        }
    }
    return next as T
}

/** Walk a ballot-style row's contests and apply the per-contest
 *  overlay from `overrides` (keyed by contest id). Returns a new
 *  ballot-style row when at least one contest changed, or the
 *  original row referentially when none did. */
export function applyPolicyOverlayToBallotStyleRow<
    T extends {
        ballot_eml?: {contests?: ContestLike[]} | null | undefined
    }
>(row: T, overrides: OverridesMap): T {
    const contests = row.ballot_eml?.contests
    if (!contests || contests.length === 0) return row
    let changed = false
    const nextContests = contests.map((c) => {
        const id = typeof c.id === "string" ? c.id : undefined
        if (!id) return c
        const overlay = overrides[id]
        if (!overlay) return c
        const merged = applyPolicyOverlayToContest(c, overlay)
        if (merged !== c) changed = true
        return merged
    })
    if (!changed) return row
    return {
        ...row,
        ballot_eml: {
            ...(row.ballot_eml as object),
            contests: nextContests,
        },
    } as T
}
