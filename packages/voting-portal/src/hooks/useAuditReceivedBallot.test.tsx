// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {renderHook} from "@testing-library/react"
import {EReceiptsPolicy, type IAuditableBallot} from "@sequentech/ui-core"
import {confirmBallotAudited} from "../store/auditableBallots/auditableBallotsSlice"
import {
    IAuditedBallot,
    isAuditedByBallotBox,
    useAuditReceivedBallot,
} from "./useAuditReceivedBallot"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("../store/hooks", () => ({
    useAppDispatch: () => mockDispatch,
}))
jest.mock("@apollo/client/react", () => ({
    useMutation: () => [mockAuditBallot],
}))

const mockDispatch = jest.fn()
const mockAuditBallot = jest.fn()
const setErrorMsg = jest.fn()

const BALLOT_HASH = "0123456789abcdef".repeat(4)
const AUDITED: IAuditedBallot = {
    electionId: "election-1",
    ballotId: "FTBE-MHRX",
    ballotHash: BALLOT_HASH,
}
const NOT_CONFIRMED = "reviewScreen.error.AUDIT_BALLOT"

const auditableBallot = (policy?: EReceiptsPolicy, isDemo?: boolean) =>
    ({
        config: {
            election_event_presentation: policy ? {receipts: {policy}} : undefined,
            public_key: {is_demo: isDemo},
        },
    }) as unknown as IAuditableBallot

const audit = () => renderHook(() => useAuditReceivedBallot()).result.current(AUDITED, setErrorMsg)

beforeEach(() => {
    jest.clearAllMocks()
    jest.spyOn(console, "log").mockImplementation(() => undefined)
    jest.spyOn(console, "error").mockImplementation(() => undefined)
    mockAuditBallot.mockResolvedValue({
        data: {audit_ballot: {ballot_id: "FTBE-MHRX", ballot_hash: BALLOT_HASH}},
    })
})
afterEach(() => jest.restoreAllMocks())

describe("isAuditedByBallotBox", () => {
    it("takes the ballot box's confirmation when it receives ballots at review", () => {
        expect(
            isAuditedByBallotBox(auditableBallot(EReceiptsPolicy.SIGNED_BY_BALLOT_BOX), false)
        ).toBe(true)
    })

    it("does not for other receipts, demo ballots, portals without authentication or no ballot", () => {
        expect(isAuditedByBallotBox(auditableBallot(), false)).toBe(false)
        expect(isAuditedByBallotBox(auditableBallot(EReceiptsPolicy.DISABLED), false)).toBe(false)
        expect(
            isAuditedByBallotBox(auditableBallot(EReceiptsPolicy.SIGNED_BY_BALLOT_BOX, true), false)
        ).toBe(false)
        expect(
            isAuditedByBallotBox(auditableBallot(EReceiptsPolicy.SIGNED_BY_BALLOT_BOX), true)
        ).toBe(false)
        expect(isAuditedByBallotBox(undefined, false)).toBe(false)
    })
})

describe("useAuditReceivedBallot", () => {
    it("asks the ballot box to mark the ballot of the receipt audited", async () => {
        expect(await audit()).toBe(true)

        expect(mockAuditBallot).toHaveBeenCalledTimes(1)
        expect(mockAuditBallot).toHaveBeenCalledWith({
            variables: {electionId: "election-1", ballotId: "FTBE-MHRX"},
        })
        expect(mockDispatch).toHaveBeenCalledWith(confirmBallotAudited("election-1"))
        expect(setErrorMsg).not.toHaveBeenCalled()
    })

    it("does not take the confirmation of another ballot", async () => {
        for (const answer of [
            {ballot_id: "0000-0000", ballot_hash: BALLOT_HASH},
            {ballot_id: "FTBE-MHRX", ballot_hash: "f".repeat(64)},
        ]) {
            mockAuditBallot.mockResolvedValueOnce({data: {audit_ballot: answer}})

            expect(await audit()).toBe(false)
        }

        expect(mockDispatch).not.toHaveBeenCalled()
        expect(setErrorMsg).toHaveBeenCalledTimes(2)
        expect(setErrorMsg).toHaveBeenCalledWith(NOT_CONFIRMED)
    })

    it("shows the audit refused when the ballot box refuses it or cannot be reached", async () => {
        mockAuditBallot.mockResolvedValueOnce({error: new Error("refused"), data: undefined})
        expect(await audit()).toBe(false)

        mockAuditBallot.mockResolvedValueOnce({data: {audit_ballot: null}})
        expect(await audit()).toBe(false)

        mockAuditBallot.mockRejectedValueOnce(new Error("network"))
        expect(await audit()).toBe(false)

        expect(mockDispatch).not.toHaveBeenCalled()
        expect(setErrorMsg).toHaveBeenCalledTimes(3)
        expect(setErrorMsg).toHaveBeenNthCalledWith(3, NOT_CONFIRMED)
    })
})
