// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {CastVoteStatus} from "../../store/castVotes/castVotesSlice"
import {IAlreadyCastInput, showsAlreadyCastHint} from "./ballotList"

const VALID = {status: CastVoteStatus.VALID}
const DISCARDED = {status: CastVoteStatus.DISCARDED}

const input = (changes: Partial<IAlreadyCastInput> = {}): IAlreadyCastInput => ({
    election: {id: "election", num_allowed_revotes: 1},
    votes: [VALID],
    isOpen: true,
    canVote: false,
    keepsTransparencyLog: true,
    ...changes,
})

describe("showsAlreadyCastHint", () => {
    it("says so when the voter cast the one ballot the election allows", () => {
        expect(showsAlreadyCastHint(input())).toBe(true)
    })

    it("says so when the voter cast every ballot of several that the election allows", () => {
        expect(
            showsAlreadyCastHint(
                input({
                    election: {id: "election", num_allowed_revotes: 3},
                    votes: [VALID, VALID, {status: CastVoteStatus.IN_PROGRESS}],
                })
            )
        ).toBe(true)
    })

    it("treats an election that sets no limit as allowing one ballot", () => {
        expect(showsAlreadyCastHint(input({election: {id: "election"}}))).toBe(true)
        expect(
            showsAlreadyCastHint(input({election: {id: "election", num_allowed_revotes: null}}))
        ).toBe(true)
    })

    it("stays silent while the voter can vote again", () => {
        expect(
            showsAlreadyCastHint(
                input({
                    election: {id: "election", num_allowed_revotes: 2},
                    canVote: true,
                })
            )
        ).toBe(false)
    })

    it("stays silent when another reason keeps a voter who has re-votes left from voting", () => {
        expect(
            showsAlreadyCastHint(
                input({election: {id: "election", num_allowed_revotes: 2}, canVote: false})
            )
        ).toBe(false)
    })

    it("stays silent when the election allows any number of ballots", () => {
        expect(
            showsAlreadyCastHint(input({election: {id: "election", num_allowed_revotes: 0}}))
        ).toBe(false)
    })

    it("stays silent for a voter who never cast a ballot, whatever keeps them from voting", () => {
        expect(showsAlreadyCastHint(input({votes: []}))).toBe(false)
        expect(
            showsAlreadyCastHint(
                input({votes: [], election: {id: "election", num_allowed_revotes: -1}})
            )
        ).toBe(false)
    })

    it("doesn't count a ballot that was discarded", () => {
        expect(showsAlreadyCastHint(input({votes: [DISCARDED]}))).toBe(false)
        expect(
            showsAlreadyCastHint(
                input({
                    election: {id: "election", num_allowed_revotes: 2},
                    votes: [VALID, DISCARDED],
                })
            )
        ).toBe(false)
    })

    it("stays silent when the election is closed, as its card has no vote button", () => {
        expect(showsAlreadyCastHint(input({isOpen: false}))).toBe(false)
    })

    it("stays silent when the voter can vote anyway, as in a preview", () => {
        expect(showsAlreadyCastHint(input({canVote: true}))).toBe(false)
    })

    it("stays silent for an event that keeps no transparency log, whose cards keep their look", () => {
        expect(showsAlreadyCastHint(input({keepsTransparencyLog: false}))).toBe(false)
    })
})
