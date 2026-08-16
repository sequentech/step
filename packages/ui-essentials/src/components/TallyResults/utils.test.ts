// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

jest.mock("@sequentech/ui-core", () => ({
    formatPercentOne: (value: number) => `${value}%`,
    TallySheetVotingChannel: {},
    VotingStatusChannel: {},
}))

import {orderCandidateReferences} from "./utils"

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
})
