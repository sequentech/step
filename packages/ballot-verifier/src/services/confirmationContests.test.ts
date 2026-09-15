// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IContest, IDecodedVoteContest} from "@sequentech/ui-core"
import {getConfirmationContests} from "./confirmationContests"

jest.mock("@sequentech/ui-core", () => ({
    isAcclaimedContest: (contest?: IContest | null) => Boolean(contest?.is_acclaimed),
}))

const contest = (id: string, isAcclaimed = false) => ({id, is_acclaimed: isAcclaimed}) as IContest

const decodedContest = (contestId: string): IDecodedVoteContest => ({
    contest_id: contestId,
    is_explicit_invalid: false,
    is_decline_to_vote: false,
    is_blank_ballot: false,
    invalid_errors: [],
    invalid_alerts: [],
    choices: [{id: `${contestId}-candidate`, selected: 0}],
})

describe("getConfirmationContests", () => {
    it("adds missing acclaimed contests in ballot-style order without changing decoded data", () => {
        const normalDecodedContest = decodedContest("normal")

        const result = getConfirmationContests(
            [contest("acclaimed", true), contest("normal")],
            [normalDecodedContest]
        )

        expect(result.map(({contest_id}) => contest_id)).toEqual(["acclaimed", "normal"])
        expect(result[0]).toEqual({
            contest_id: "acclaimed",
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: [],
            invalid_alerts: [],
            choices: [],
        })
        expect(result[1]).toBe(normalDecodedContest)
    })

    it("does not duplicate an acclaimed contest already present in decoded data", () => {
        const decodedAcclaimedContest = decodedContest("acclaimed")

        const result = getConfirmationContests(
            [contest("acclaimed", true)],
            [decodedAcclaimedContest]
        )

        expect(result).toEqual([decodedAcclaimedContest])
    })

    it("preserves unconfigured decoded entries after configured contests", () => {
        const unknownDecodedContest = decodedContest("unknown")
        const normalDecodedContest = decodedContest("normal")

        const result = getConfirmationContests(
            [contest("normal"), contest("acclaimed", true)],
            [unknownDecodedContest, normalDecodedContest]
        )

        expect(result.map(({contest_id}) => contest_id)).toEqual(["normal", "acclaimed", "unknown"])
        expect(result[2]).toBe(unknownDecodedContest)
    })
})
