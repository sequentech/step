// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

jest.mock(
    "@sequentech/ui-core",
    () => ({
        formatPercentOne: (value: number) => `${value}%`,
        TallySheetVotingChannel: {},
        VotingStatusChannel: {},
    }),
    {virtual: true}
)

import {buildCandidateChartData, mergeLabels, orderCandidateReferences} from "./utils"

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
