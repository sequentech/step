// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {RootState} from "../store"
import {canVoteSomeElection} from "./castVotesSlice"

const state = (completed: boolean): RootState =>
    ({
        ballotStyles: {election: {}},
        elections: {
            election: {id: "election", num_allowed_revotes: 0},
        },
        castVotes: {},
        extra: {
            completedAcclaimedElections: completed ? {election: true} : {},
        },
    }) as unknown as RootState

describe("canVoteSomeElection", () => {
    it("does not offer a completed fully acclaimed election", () => {
        expect(canVoteSomeElection()(state(true))).toBe(false)
    })

    it("continues to offer an otherwise available election", () => {
        expect(canVoteSomeElection()(state(false))).toBe(true)
    })

    it("continues to offer another election after acclaim completion", () => {
        const multipleElections = {
            ...state(true),
            ballotStyles: {
                election: {},
                other: {},
            },
            elections: {
                election: {id: "election", num_allowed_revotes: 0},
                other: {id: "other", num_allowed_revotes: 1},
            },
        } as unknown as RootState

        expect(canVoteSomeElection()(multipleElections)).toBe(true)
    })

    it("tolerates preloaded state from before acclaim completion existed", () => {
        const legacyState = state(false)
        delete (legacyState.extra as Partial<typeof legacyState.extra>).completedAcclaimedElections

        expect(canVoteSomeElection()(legacyState)).toBe(true)
    })
})
