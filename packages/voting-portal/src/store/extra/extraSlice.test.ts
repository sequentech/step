// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import reducer, {completeAcclaimedElection} from "./extraSlice"

describe("extraSlice", () => {
    it("records acclaimed completion separately from a cast vote", () => {
        const state = reducer(undefined, completeAcclaimedElection("election"))

        expect(state.completedAcclaimedElections).toEqual({election: true})
        expect(state.isVoted).toEqual({})
    })

    it("keeps acclaimed completion when transient ballot state is cleared", () => {
        const completed = reducer(undefined, completeAcclaimedElection("election"))
        const state = reducer(completed, {type: "extra/clearIsVoted"})

        expect(state.completedAcclaimedElections).toEqual({election: true})
    })
})
