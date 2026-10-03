// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {renderHook} from "@testing-library/react"
import type {IReceivedBallot} from "@sequentech/ui-core"
import {addCastVotes} from "../store/castVotes/castVotesSlice"
import {IReceivedCast, signReceivedCast, useCastReceivedBallot} from "./useCastReceivedBallot"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, string>) =>
            values ? `${key} ${values.ballotId} ${values.auditableBallotHash}` : key,
    }),
}))
jest.mock("../store/hooks", () => ({
    useAppDispatch: () => mockDispatch,
}))
jest.mock("@apollo/client/react", () => ({
    useMutation: () => [mockCastBallot],
}))
jest.mock("../services/BallotService", () => ({
    provideBallotService: () => ({
        signBallotCast: (...args: unknown[]) => mockSignBallotCast(...args),
        verifyCastReceipt: (...args: unknown[]) => mockVerifyCastReceipt(...args),
        forgetVoterSigningKey: (...args: unknown[]) => mockForgetVoterSigningKey(...args),
    }),
}))

const mockDispatch = jest.fn()
const mockCastBallot = jest.fn()
const mockSignBallotCast = jest.fn()
const mockVerifyCastReceipt = jest.fn()
const mockForgetVoterSigningKey = jest.fn()
const setErrorMsg = jest.fn()

const BALLOT_HASH = "0123456789abcdef".repeat(4)
const BALLOT_BOX_KEY = {key_id: "fd110d301d2f077d", public_key: "ballot-box-key"}
const RECEIVED: IReceivedBallot = {
    tenant_id: "tenant-1",
    election_event_id: "event-1",
    election_id: "election-1",
    ballot_hash: BALLOT_HASH,
    voter_signing_pk: "voter-key",
    voter_ballot_signature: "voter-signature",
    received_at: "2028-05-08T03:00:00.000Z",
    key_id: "fd110d301d2f077d",
    received_signature: "ballot-box-signature",
    ballot_id: "FTBE-MHRX",
}
const CAST: IReceivedCast = {
    receivedBallot: RECEIVED,
    ballotBoxKey: BALLOT_BOX_KEY,
    castSignature: "cast-signature",
}
const ANSWER = {
    cast_vote_id: "cast-vote-1",
    cast_at: "2028-05-08T03:04:05.678Z",
    key_id: "fd110d301d2f077d",
    cast_receipt_signature: "ballot-box-cast-signature",
}

const cast = () => renderHook(() => useCastReceivedBallot()).result.current(CAST, setErrorMsg)

beforeEach(() => {
    jest.clearAllMocks()
    jest.spyOn(console, "log").mockImplementation(() => undefined)
    jest.spyOn(console, "error").mockImplementation(() => undefined)
    mockCastBallot.mockResolvedValue({data: {cast_ballot: ANSWER}})
    mockVerifyCastReceipt.mockReturnValue(true)
})
afterEach(() => jest.restoreAllMocks())

it("signs the cast of the Ballot ID with the key that signed the ballot", () => {
    mockSignBallotCast.mockReturnValue("cast-signature")

    expect(signReceivedCast(RECEIVED, BALLOT_BOX_KEY)).toEqual(CAST)
    expect(mockSignBallotCast).toHaveBeenCalledWith("election-1", "voter-key", "FTBE-MHRX")
})

it("does not make up a cast when the key that signed the ballot is gone", () => {
    const gone = new Error("The key that signed the ballot is no longer in memory")
    mockSignBallotCast.mockImplementation(() => {
        throw gone
    })

    expect(() => signReceivedCast(RECEIVED, BALLOT_BOX_KEY)).toThrow(gone)
})

it("casts by the Ballot ID and the signature, without sending the ballot again", async () => {
    expect(await cast()).toBe(true)

    expect(mockCastBallot).toHaveBeenCalledTimes(1)
    expect(mockCastBallot).toHaveBeenCalledWith({
        variables: {
            electionId: "election-1",
            ballotId: "FTBE-MHRX",
            castSignature: "cast-signature",
        },
    })
    expect(setErrorMsg).not.toHaveBeenCalled()
})

it("checks the receipt against what this device asked to cast, not what the answer says", async () => {
    mockCastBallot.mockResolvedValue({
        data: {
            cast_ballot: {
                ...ANSWER,
                ballot_id: "0000-0000",
                election_id: "another-election",
                cast_signature: "another-signature",
                received_at: "2028-05-08T02:00:00.000Z",
            },
        },
    })

    expect(await cast()).toBe(true)
    expect(mockVerifyCastReceipt).toHaveBeenCalledWith(BALLOT_BOX_KEY, {
        election_event_id: "event-1",
        election_id: "election-1",
        ballot_id: "FTBE-MHRX",
        received_at: "2028-05-08T03:00:00.000Z",
        cast_at: "2028-05-08T03:04:05.678Z",
        key_id: "fd110d301d2f077d",
        cast_signature: "cast-signature",
        cast_receipt_signature: "ballot-box-cast-signature",
    })
})

it("records the cast vote and drops the voter's key once the receipt verifies", async () => {
    expect(await cast()).toBe(true)

    expect(mockDispatch).toHaveBeenCalledWith(
        addCastVotes([
            {
                id: "cast-vote-1",
                tenant_id: "tenant-1",
                election_id: "election-1",
                election_event_id: "event-1",
                created_at: "2028-05-08T03:04:05.678Z",
            },
        ])
    )
    expect(mockForgetVoterSigningKey).toHaveBeenCalledWith("election-1")
})

it("does not confirm a cast whose receipt the ballot box did not sign", async () => {
    mockVerifyCastReceipt.mockImplementation(() => {
        throw new Error("The ballot box signature does not verify")
    })

    expect(await cast()).toBe(false)
    expect(setErrorMsg).toHaveBeenCalledWith(
        `reviewScreen.error.INCONSISTENT_HASH FTBE-MHRX ${BALLOT_HASH}`
    )
    expect(mockDispatch).not.toHaveBeenCalled()
    // The voter can ask again with the same key: a retry returns the receipt.
    expect(mockForgetVoterSigningKey).not.toHaveBeenCalled()
})

it.each([
    ["no answer", {data: {cast_ballot: null}}],
    ["an error result", {data: undefined, error: new Error("refused")}],
])("shows the fetch error and confirms nothing for %s", async (_, result) => {
    mockCastBallot.mockResolvedValue(result)

    expect(await cast()).toBe(false)
    expect(setErrorMsg).toHaveBeenCalledWith("reviewScreen.error.UNABLE_TO_FETCH_DATA")
    expect(mockVerifyCastReceipt).not.toHaveBeenCalled()
    expect(mockDispatch).not.toHaveBeenCalled()
})

it("shows the ballot box's refusal with the cast error texts", async () => {
    mockCastBallot.mockRejectedValue({
        message: "refused",
        graphQLErrors: [{extensions: {code: "PokValidationFailed"}}],
    })

    expect(await cast()).toBe(false)
    expect(setErrorMsg).toHaveBeenCalledWith("reviewScreen.error.CAST_VOTE_PokValidationFailed")
    expect(mockDispatch).not.toHaveBeenCalled()
})

it("shows the generic cast error when the request fails without a reason", async () => {
    mockCastBallot.mockRejectedValue(new Error("boom"))

    expect(await cast()).toBe(false)
    expect(setErrorMsg).toHaveBeenCalledWith("reviewScreen.error.CAST_VOTE")
})
