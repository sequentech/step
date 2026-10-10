// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    CandidatesOrder,
    EBlankVotePolicy,
    ECandidatesSelectionPolicy,
    EDuplicatedRankPolicy,
    EInvalidVotePolicy,
    EOverVotePolicy,
    EPreferenceGapsPolicy,
    EUnderVotePolicy,
} from "@sequentech/ui-core"
import type {
    JsonObject,
    PreviewContest,
    ScenarioSnapshot,
} from "@sequentech/ui-test-kit/fixtures/scenarios"

/** Contest presentation policies the workbench can override. */
export interface ContestPolicies {
    invalid_vote_policy?: EInvalidVotePolicy
    blank_vote_policy?: EBlankVotePolicy
    over_vote_policy?: EOverVotePolicy
    under_vote_policy?: EUnderVotePolicy
    duplicated_rank_policy?: EDuplicatedRankPolicy
    preference_gaps_policy?: EPreferenceGapsPolicy
    candidates_order?: CandidatesOrder
    candidates_selection_policy?: ECandidatesSelectionPolicy
}

export type PolicyKey = keyof ContestPolicies

export enum BoundKey {
    MIN = "min_votes",
    MAX = "max_votes",
}

/** Overrides of one contest: presentation policies and the vote bounds that frame them. */
export type ContestOverrides = ContestPolicies & Partial<Record<BoundKey, number>>

/** Overrides by contest ID. The snapshot keeps its own values; these are applied on top. */
export type PolicyOverrides = Readonly<Record<string, ContestOverrides>>

export enum PolicyKind {
    VALIDATION = "validation",
    PRESENTATION = "presentation",
}

/** Which contests a policy affects. */
export enum PolicyScope {
    ALL = "all",
    PREFERENTIAL = "preferential",
}

export interface PolicyField {
    key: PolicyKey
    label: string
    kind: PolicyKind
    scope: PolicyScope
    values: readonly string[]
}

export const POLICY_FIELDS: readonly PolicyField[] = [
    {
        key: "invalid_vote_policy",
        label: "Invalid vote",
        kind: PolicyKind.VALIDATION,
        scope: PolicyScope.ALL,
        values: Object.values(EInvalidVotePolicy),
    },
    {
        key: "blank_vote_policy",
        label: "Blank vote",
        kind: PolicyKind.VALIDATION,
        scope: PolicyScope.ALL,
        values: Object.values(EBlankVotePolicy),
    },
    {
        key: "over_vote_policy",
        label: "Over vote",
        kind: PolicyKind.VALIDATION,
        scope: PolicyScope.ALL,
        values: Object.values(EOverVotePolicy),
    },
    {
        key: "under_vote_policy",
        label: "Under vote",
        kind: PolicyKind.VALIDATION,
        scope: PolicyScope.ALL,
        values: Object.values(EUnderVotePolicy),
    },
    {
        key: "duplicated_rank_policy",
        label: "Duplicated rank",
        kind: PolicyKind.VALIDATION,
        scope: PolicyScope.PREFERENTIAL,
        values: Object.values(EDuplicatedRankPolicy),
    },
    {
        key: "preference_gaps_policy",
        label: "Preference gaps",
        kind: PolicyKind.VALIDATION,
        scope: PolicyScope.PREFERENTIAL,
        values: Object.values(EPreferenceGapsPolicy),
    },
    {
        key: "candidates_order",
        label: "Candidates order",
        kind: PolicyKind.PRESENTATION,
        scope: PolicyScope.ALL,
        values: Object.values(CandidatesOrder),
    },
    {
        key: "candidates_selection_policy",
        label: "Selection",
        kind: PolicyKind.PRESENTATION,
        scope: PolicyScope.ALL,
        values: Object.values(ECandidatesSelectionPolicy),
    },
]

const BOUNDS = Object.values(BoundKey)
const isBound = (key: string): key is BoundKey => BOUNDS.includes(key as BoundKey)

export const policyField = (key: string) => POLICY_FIELDS.find((field) => field.key === key)

/** The contest's own presentation policies, as the snapshot stores them. */
export function contestPolicies(contest: PreviewContest): ContestPolicies {
    const presentation = (contest.presentation ?? {}) as JsonObject
    return Object.fromEntries(
        POLICY_FIELDS.flatMap(({key, values}) =>
            values.includes(presentation[key] as string) ? [[key, presentation[key]]] : []
        )
    ) as ContestPolicies
}

/** Why a contest's effective vote bounds cannot be used, if they cannot. */
export function boundsIssue(contest: PreviewContest, overrides: ContestOverrides = {}) {
    const min = overrides[BoundKey.MIN] ?? contest.min_votes
    const max = overrides[BoundKey.MAX] ?? contest.max_votes
    if (![min, max].every((bound) => Number.isInteger(bound) && bound >= 0))
        return "Bounds are non-negative whole numbers"
    if (max < 1) return "The maximum must allow at least one vote"
    if (min > max) return "The minimum exceeds the maximum"
    return undefined
}

/** Unusable bounds stay at the contest's values; its policies still apply. */
function applyContest(contest: PreviewContest, overrides?: ContestOverrides): PreviewContest {
    if (!overrides) return contest
    const policies = Object.fromEntries(
        POLICY_FIELDS.flatMap(({key}) => (overrides[key] ? [[key, overrides[key]]] : []))
    ) as JsonObject
    const bounds = boundsIssue(contest, overrides)
        ? {}
        : (Object.fromEntries(
              BOUNDS.flatMap((key) => (overrides[key] === undefined ? [] : [[key, overrides[key]]]))
          ) as JsonObject)
    return {
        ...contest,
        ...bounds,
        presentation: {...((contest.presentation ?? {}) as JsonObject), ...policies},
    }
}

/** The snapshot with the overrides applied to every ballot style's contests. */
export function applyPolicyOverrides(
    snapshot: ScenarioSnapshot,
    overrides: PolicyOverrides
): ScenarioSnapshot {
    if (!Object.keys(overrides).length) return snapshot
    return {
        ...snapshot,
        preview: {
            ...snapshot.preview,
            ballot_styles: snapshot.preview.ballot_styles.map((style) => ({
                ...style,
                contests: style.contests.map((contest) =>
                    applyContest(contest, overrides[contest.id])
                ),
            })),
        },
    }
}

/** One line per overridden value, naming the contest; used as snapshot provenance. */
export function describePolicyOverrides(
    snapshot: ScenarioSnapshot,
    overrides: PolicyOverrides
): string[] {
    const contests = new Map(
        snapshot.preview.ballot_styles.flatMap(({contests}) =>
            contests.map((contest) => [contest.id, contest] as const)
        )
    )
    return Object.entries(overrides).flatMap(([id, contestOverrides]) =>
        Object.entries(contestOverrides).map(
            ([key, value]) => `${String(contests.get(id)?.name ?? id)}: ${key} = ${String(value)}`
        )
    )
}

/** A copy with one value set, or cleared when `value` is undefined; empty contests drop out. */
export function withOverride(
    overrides: PolicyOverrides,
    contestId: string,
    key: PolicyKey | BoundKey,
    value: string | number | undefined
): PolicyOverrides {
    const contest: Record<string, string | number> = {...overrides[contestId]}
    if (value === undefined) delete contest[key]
    else contest[key] = value
    const next: Record<string, ContestOverrides> = {...overrides}
    if (Object.keys(contest).length) next[contestId] = contest as ContestOverrides
    else delete next[contestId]
    return next
}

/** Reads stored overrides, rejecting unknown keys and values outside each policy's enum. */
export function parsePolicyOverrides(value: unknown): PolicyOverrides {
    if (typeof value !== "object" || value === null || Array.isArray(value))
        throw new Error("Policy overrides must be an object of contests")
    for (const [contestId, contest] of Object.entries(value)) {
        if (typeof contest !== "object" || contest === null || Array.isArray(contest))
            throw new Error(`Overrides of contest ${contestId} must be an object`)
        for (const [key, setting] of Object.entries(contest as Record<string, unknown>)) {
            const field = policyField(key)
            if (field ? !field.values.includes(setting as string) : !isBound(key))
                throw new Error(`Unsupported override ${key} = ${String(setting)}`)
            if (!field && !(Number.isInteger(setting) && (setting as number) >= 0))
                throw new Error(`Bound ${key} must be a non-negative whole number`)
        }
    }
    return value as PolicyOverrides
}
