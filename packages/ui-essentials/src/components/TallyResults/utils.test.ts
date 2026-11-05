// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

jest.mock(
    "@sequentech/ui-core",
    () => ({
        formatPercentOne: (value: number) => `${value}%`,
        ...jest.requireActual<typeof import("../../../../ui-core/src/types/VotingChannel")>(
            "../../../../ui-core/src/types/VotingChannel"
        ),
    }),
    {virtual: true}
)

import {
    buildCandidateChartData,
    mergeLabels,
    orderCandidateReferences,
    percentOrDash,
    sortCandidateResults,
    toFiniteNumber,
    valueOrDash,
} from "./utils"

describe("orderCandidateReferences", () => {
    it("uses configured candidate order and retains process-only references", () => {
        const references = [
            {id: "candidate-a", name: "A candidate"},
            {id: "process-only", name: "Process only"},
            {id: "candidate-z", name: "Z candidate"},
        ]
        const candidates = [
            {id: "candidate-z", name: "Z candidate"},
            {id: "candidate-a", name: "A candidate"},
        ]

        expect(
            orderCandidateReferences(references, candidates).map((reference) => reference.id)
        ).toEqual(["candidate-z", "candidate-a", "process-only"])
    })

    it("handles empty inputs and candidate IDs without references", () => {
        const references = [
            {id: "candidate-a", name: "A candidate"},
            {id: "process-only", name: "Process only"},
        ]

        expect(orderCandidateReferences([], [])).toEqual([])
        expect(orderCandidateReferences(references, [])).toEqual(references)
        expect(
            orderCandidateReferences(references, [
                {id: "missing-reference", name: "Missing reference"},
                {id: "candidate-a", name: "A candidate"},
            ]).map((reference) => reference.id)
        ).toEqual(["candidate-a", "process-only"])
    })
})

describe("buildCandidateChartData", () => {
    it("preserves the configured candidate input order instead of ranking by votes", () => {
        const chartData = buildCandidateChartData(
            [
                {id: "configured-first", name: "Configured first", castVotes: 1},
                {id: "configured-second", name: "Configured second", castVotes: 100},
            ],
            mergeLabels()
        )

        expect(chartData.map((item) => item.label)).toEqual([
            "Configured first",
            "Configured second",
        ])
    })
})

// These assertions use raw result values. Rendering a plausible chart must not
// turn missing data into a measured zero or reorder the configured candidates.

it.each([undefined, null, "", "  ", "not a number", "NaN", "Infinity", NaN, Infinity, -Infinity])(
    "keeps a missing or non-finite tally value unavailable (%s)",
    (value) => {
        expect(toFiniteNumber(value)).toBeNull()
        expect(valueOrDash(value)).toBe("-")
        expect(percentOrDash(value)).toBe("-")
    }
)

it.each([
    [0, 0],
    ["0", 0],
    [" 42.5 ", 42.5],
    [-3, -3],
    ["1e2", 100],
] as const)("preserves the numeric value of %s, including zero", (value, expected) => {
    expect(toFiniteNumber(value)).toBe(expected)
    expect(valueOrDash(value)).toBe(expected)
    expect(percentOrDash(value)).toBe(`${expected}%`)
})

it("groups only positive votes after the fifth candidate and preserves the inputs", () => {
    const results = [
        {id: "zero", name: "Zero", castVotes: 0},
        {id: "invalid", name: "Unavailable", castVotes: "bad"},
        ...[1, 2, 3, 4, 5, 6, 7].map((votes) => ({
            id: String(votes),
            name: `Candidate ${votes}`,
            castVotes: votes,
        })),
    ]
    const before = JSON.stringify(results)
    expect(buildCandidateChartData(results, mergeLabels({others: "Remaining candidates"}))).toEqual(
        [
            {label: "Candidate 1", value: 1},
            {label: "Candidate 2", value: 2},
            {label: "Candidate 3", value: 3},
            {label: "Candidate 4", value: 4},
            {label: "Candidate 5", value: 5},
            {label: "Remaining candidates", value: 13},
        ]
    )
    expect(JSON.stringify(results)).toBe(before)
    expect(
        buildCandidateChartData([{id: "unnamed", name: "", castVotes: 2}], mergeLabels())
    ).toEqual([{label: "-", value: 2}])
})

it("uses winning position before vote count and sorts unavailable positions last", () => {
    const results = [
        {id: "unranked", name: "Unranked", castVotes: 10_000},
        {id: "second", name: "Second", castVotes: 9_000, winningPosition: 2},
        {id: "first-low", name: "First low", castVotes: 20, winningPosition: 1},
        {id: "first-high", name: "First high", castVotes: 30, winningPosition: "1"},
        {id: "missing", name: "Missing"},
    ]
    expect(results.sort(sortCandidateResults).map(({id}) => id)).toEqual([
        "first-high",
        "first-low",
        "second",
        "unranked",
        "missing",
    ])
    expect(sortCandidateResults({id: "a", name: "A"}, {id: "b", name: "B"})).toBe(0)
})

it("merges one translated channel name without removing the other defaults", () => {
    const defaults = mergeLabels()
    const [channel, name] = Object.entries(defaults.channelNames)[0]
    expect(name).toBeDefined()
    const override = {[channel]: "Translated channel"}
    const merged = mergeLabels({channelNames: override})
    expect(merged.channelNames).toEqual({...defaults.channelNames, ...override})
    expect(mergeLabels().channelNames).toEqual(defaults.channelNames)
})
