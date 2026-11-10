// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IAuditableBallot} from "@sequentech/ui-core"
import type {RootState} from "../store"
import reducer, {
    AuditableBallotsState,
    confirmBallotAudited,
    markBallotAudited,
    selectIsBallotAuditConfirmed,
    selectIsBallotAudited,
    setAuditableBallot,
} from "./auditableBallotsSlice"

const ballot = (ballotHash: string) => ({ballot_hash: ballotHash}) as IAuditableBallot

const isAudited = (state: AuditableBallotsState, electionId = "election") =>
    selectIsBallotAudited(electionId)({auditableBallots: state} as unknown as RootState)

const isAuditConfirmed = (state: AuditableBallotsState, electionId = "election") =>
    selectIsBallotAuditConfirmed(electionId)({auditableBallots: state} as unknown as RootState)

const encrypted = (ballotHash: string, state?: AuditableBallotsState) =>
    reducer(
        state,
        setAuditableBallot({
            electionId: "election",
            auditableBallot: ballot(ballotHash),
            isBlankBallot: false,
        })
    )

describe("auditableBallotsSlice", () => {
    it("does not take a newly encrypted ballot for an audited one", () => {
        expect(isAudited(encrypted("first"))).toBe(false)
    })

    it("marks the ballot of one election as audited", () => {
        const state = reducer(encrypted("first"), markBallotAudited("election"))

        expect(isAudited(state)).toBe(true)
        expect(state.election?.auditableBallot.ballot_hash).toBe("first")
        expect(isAudited(state, "other")).toBe(false)
    })

    it("clears the mark when the next ballot is encrypted", () => {
        const audited = reducer(encrypted("first"), markBallotAudited("election"))
        const state = encrypted("second", audited)

        expect(isAudited(state)).toBe(false)
        expect(state.election?.auditableBallot.ballot_hash).toBe("second")
    })

    it("marks nothing when there is no ballot", () => {
        expect(reducer(undefined, markBallotAudited("election"))).toEqual({})
    })

    it("keeps the ballot box's confirmation of an audit for that ballot only", () => {
        expect(isAuditConfirmed(encrypted("first"))).toBe(false)

        const confirmed = reducer(encrypted("first"), confirmBallotAudited("election"))
        expect(isAuditConfirmed(confirmed)).toBe(true)
        expect(isAudited(confirmed)).toBe(false)
        expect(isAuditConfirmed(confirmed, "other")).toBe(false)

        expect(isAuditConfirmed(encrypted("second", confirmed))).toBe(false)
        expect(reducer(undefined, confirmBallotAudited("election"))).toEqual({})
    })
})
