// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// ui-core's built dist is unavailable when this package's tests run alone, so
// load the number formatting sources the helpers write figures with.
jest.mock(
    "@sequentech/ui-core",
    () => ({
        ...jest.requireActual<typeof import("../../../../ui-core/src/types/VotingChannel")>(
            "../../../../ui-core/src/types/VotingChannel"
        ),
        ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
        ...jest.requireActual("../../../../ui-core/src/services/numberFormat"),
        ...jest.requireActual("../../../../ui-core/src/services/percentFormatter"),
    }),
    {virtual: true}
)

import {ENumberFormatPolicy} from "@sequentech/ui-core"
import {
    buildCandidateChartData,
    mergeLabels,
    orderCandidateReferences,
    percentOrDash,
    pieChartNumberFormatOptions,
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
})

it.each([0, "0"])("writes a zero tally value as a figure, not a dash (%p)", (value) => {
    expect(valueOrDash(value)).toBe("0")
    expect(percentOrDash(value)).toBe("0.00%")
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

describe("valueOrDash", () => {
    it("groups counts in thousands with the event's number format", () => {
        expect(valueOrDash(12000000, ENumberFormatPolicy.PERIOD_COMMA)).toBe("12.000.000")
        expect(valueOrDash(8589934591, ENumberFormatPolicy.SPACE_COMMA)).toBe(
            "8\u00a0589\u00a0934\u00a0591"
        )
        expect(valueOrDash("1234", ENumberFormatPolicy.APOSTROPHE_PERIOD)).toBe("1\u2019234")
        expect(valueOrDash(0, ENumberFormatPolicy.PERIOD_COMMA)).toBe("0")
    })

    it("uses comma grouping for events without a number format", () => {
        expect(valueOrDash(12000000)).toBe("12,000,000")
        expect(valueOrDash(12000000, null)).toBe("12,000,000")
        expect(valueOrDash(999)).toBe("999")
    })

    it("keeps counts given as text exact beyond 2^53", () => {
        expect(valueOrDash("9007199254740993")).toBe("9,007,199,254,740,993")
    })

    it("writes a dash for missing or non-numeric values", () => {
        expect(valueOrDash(null)).toBe("-")
        expect(valueOrDash(undefined)).toBe("-")
        expect(valueOrDash("")).toBe("-")
        expect(valueOrDash("n/a")).toBe("-")
        expect(valueOrDash(Number.NaN)).toBe("-")
        expect(valueOrDash(Number.POSITIVE_INFINITY)).toBe("-")
    })
})

describe("percentOrDash", () => {
    it("writes a fraction as a percentage with two decimals in the event's number format", () => {
        expect(percentOrDash(0.456789, ENumberFormatPolicy.PERIOD_COMMA)).toBe("45,68%")
        expect(percentOrDash("0.5", ENumberFormatPolicy.SPACE_COMMA)).toBe("50,00%")
        expect(percentOrDash(1, ENumberFormatPolicy.APOSTROPHE_PERIOD)).toBe("100.00%")
    })

    it("uses the default format for events without a number format", () => {
        expect(percentOrDash(0.456789)).toBe("45.68%")
        expect(percentOrDash(0)).toBe("0.00%")
    })

    it("writes a dash for missing or non-numeric values", () => {
        expect(percentOrDash(null)).toBe("-")
        expect(percentOrDash(undefined)).toBe("-")
        expect(percentOrDash("")).toBe("-")
        expect(percentOrDash(Number.NaN)).toBe("-")
    })
})

describe("pieChartNumberFormatOptions", () => {
    it("labels slices and tooltips in the event's number format", () => {
        const options = pieChartNumberFormatOptions(ENumberFormatPolicy.PERIOD_COMMA)

        expect(options.dataLabels.formatter(45.678)).toBe("45,7%")
        expect(options.tooltip.y.formatter(12000000)).toBe("12.000.000")
    })

    it("keeps the chart library's one-decimal slice labels without a number format", () => {
        const options = pieChartNumberFormatOptions()

        expect(options.dataLabels.formatter(45.678)).toBe(`${(45.678).toFixed(1)}%`)
        expect(options.dataLabels.formatter(100)).toBe("100.0%")
        expect(options.tooltip.y.formatter(12000000)).toBe("12,000,000")
    })
})
