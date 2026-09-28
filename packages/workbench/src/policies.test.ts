// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {describe, expect, test} from "vitest"
import {EDuplicatedRankPolicy, EOverVotePolicy} from "@sequentech/ui-core"
import {RANKED_IDS, ScenarioId, scenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {
    applyPolicyOverrides,
    BoundKey,
    boundsIssue,
    contestPolicies,
    describePolicyOverrides,
    parsePolicyOverrides,
    POLICY_FIELDS,
    withOverride,
} from "./policies"

const ranked = () => scenarioSnapshot(ScenarioId.RANKED_MULTI_CONTEST)
const contests = (snapshot = ranked()) => snapshot.preview.ballot_styles[0].contests

test("the fields cover each policy enum and mark the ranking-only ones", () => {
    expect(POLICY_FIELDS.map(({key, scope, values}) => [key, scope, values.length])).toEqual([
        ["invalid_vote_policy", "all", 5],
        ["blank_vote_policy", "all", 4],
        ["over_vote_policy", "all", 5],
        ["under_vote_policy", "all", 4],
        ["duplicated_rank_policy", "preferential", 2],
        ["preference_gaps_policy", "preferential", 2],
        ["candidates_order", "all", 3],
        ["candidates_selection_policy", "all", 2],
    ])
})

test("a contest's own policies ignore values outside the policy enums", () => {
    const [council] = contests()
    expect(contestPolicies(council)).toEqual({
        invalid_vote_policy: "not-allowed",
        candidates_order: "custom",
    })
    council.presentation = {over_vote_policy: "sometimes", blank_vote_policy: "warn"}
    expect(contestPolicies(council)).toEqual({blank_vote_policy: "warn"})
    delete council.presentation
    expect(contestPolicies(council)).toEqual({})
})

describe("applying overrides", () => {
    test("replaces the overridden contest's policies and bounds only", () => {
        const snapshot = ranked()
        const applied = applyPolicyOverrides(snapshot, {
            [RANKED_IDS.contest]: {
                duplicated_rank_policy: EDuplicatedRankPolicy.NOT_ALLOWED_WARN_AND_DIALOG,
                max_votes: 2,
            },
        })
        const [council, budget] = contests(applied)
        expect(council).toBe(contests(snapshot)[0])
        expect(budget).toMatchObject({
            min_votes: 0,
            max_votes: 2,
            presentation: {
                candidates_order: "custom",
                invalid_vote_policy: "not-allowed",
                sort_order: 1,
                duplicated_rank_policy: "not-allowed-warn-and-dialog",
            },
        })
        // The loaded snapshot keeps its own values.
        expect(contests(snapshot)[1].max_votes).toBe(3)
        expect(applied.provenance).toBe(snapshot.provenance)
    })

    test("without overrides returns the snapshot itself", () => {
        const snapshot = ranked()
        expect(applyPolicyOverrides(snapshot, {})).toBe(snapshot)
    })

    test("keeps the contest bounds when the overridden ones are unusable", () => {
        const applied = applyPolicyOverrides(ranked(), {
            [IDS.contest]: {min_votes: 2, over_vote_policy: EOverVotePolicy.ALLOWED},
        })
        expect(contests(applied)[0]).toMatchObject({
            min_votes: 1,
            max_votes: 1,
            presentation: {over_vote_policy: "allowed"},
        })
    })
})

test("unusable bounds are explained", () => {
    const [council, budget] = contests()
    expect(boundsIssue(council)).toBeUndefined()
    expect(boundsIssue(council, {[BoundKey.MIN]: 2})).toBe("The minimum exceeds the maximum")
    expect(boundsIssue(budget, {[BoundKey.MAX]: 0})).toBe(
        "The maximum must allow at least one vote"
    )
    expect(boundsIssue(budget, {[BoundKey.MIN]: 1.5})).toBe("Bounds are non-negative whole numbers")
    expect(boundsIssue(budget, {[BoundKey.MIN]: 3, [BoundKey.MAX]: 3})).toBeUndefined()
})

test("each override is described with its contest name", () => {
    expect(
        describePolicyOverrides(ranked(), {
            [IDS.contest]: {over_vote_policy: EOverVotePolicy.ALLOWED},
            [RANKED_IDS.contest]: {max_votes: 2},
            "unknown-contest": {min_votes: 0},
        })
    ).toEqual([
        "Council representative: over_vote_policy = allowed",
        "Budget priorities: max_votes = 2",
        "unknown-contest: min_votes = 0",
    ])
})

test("setting and clearing values keeps the other contests and drops empty ones", () => {
    const one = withOverride({}, "a", "blank_vote_policy", "warn")
    const two = withOverride(one, "b", BoundKey.MAX, 4)
    expect(two).toEqual({a: {blank_vote_policy: "warn"}, b: {max_votes: 4}})
    expect(one).toEqual({a: {blank_vote_policy: "warn"}})
    expect(withOverride(two, "a", "blank_vote_policy", undefined)).toEqual({b: {max_votes: 4}})
})

describe("parsing stored overrides", () => {
    test("accepts policy enum values and whole-number bounds", () => {
        const stored = {[IDS.contest]: {under_vote_policy: "warn", min_votes: 0, max_votes: 2}}
        expect(parsePolicyOverrides(stored)).toBe(stored)
        expect(parsePolicyOverrides({})).toEqual({})
    })

    test.each([
        [[], "Policy overrides must be an object of contests"],
        [null, "Policy overrides must be an object of contests"],
        [{c: "warn"}, "Overrides of contest c must be an object"],
        [
            {c: {under_vote_policy: "sometimes"}},
            "Unsupported override under_vote_policy = sometimes",
        ],
        [{c: {logo_url: "x"}}, "Unsupported override logo_url = x"],
        [{c: {max_votes: -1}}, "Bound max_votes must be a non-negative whole number"],
        [{c: {min_votes: "1"}}, "Bound min_votes must be a non-negative whole number"],
    ])("rejects %j", (value, message) => {
        expect(() => parsePolicyOverrides(value)).toThrow(message)
    })
})
