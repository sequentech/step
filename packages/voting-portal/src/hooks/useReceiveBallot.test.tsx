// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {renderHook, waitFor} from "@testing-library/react"
import {EReceiptsPolicy} from "@sequentech/ui-core"
import type {IAuditableBallot, IReceivedBallot} from "@sequentech/ui-core"
import type {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import type {RootState} from "../store/store"
import {setReceivedBallot} from "../store/receivedBallots/receivedBallotsSlice"
import {EReceiveBallotStatus, useReceiveBallot} from "./useReceiveBallot"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, string>) =>
            values ? `${key} ${values.ballotId} ${values.auditableBallotHash}` : key,
    }),
}))
jest.mock("../store/hooks", () => ({
    useAppSelector: (selector: (state: RootState) => unknown) => selector(mockState),
    useAppDispatch: () => mockDispatch,
}))
jest.mock("@apollo/client/react", () => ({
    useMutation: () => [mockReceiveBallot],
}))
jest.mock("../services/BallotService", () => ({
    provideBallotService: () => ({
        toHashableBallot: (ballot: IAuditableBallot) => ({single: ballot.ballot_hash}),
        toHashableMultiBallot: (ballot: IAuditableBallot) => ({multi: ballot.ballot_hash}),
        verifyReceivedBallot: (...args: unknown[]) => mockVerifyReceivedBallot(...args),
    }),
}))

const mockDispatch = jest.fn()
const mockReceiveBallot = jest.fn()
const mockVerifyReceivedBallot = jest.fn()
let mockState: RootState

const BALLOT_HASH = "0123456789abcdef".repeat(4)
const BALLOT_BOX_KEY = {key_id: "fd110d301d2f077d", public_key: "ballot-box-key"}
const ANSWER = {
    ballot_id: "FTBE-MHRX",
    received_at: "2028-05-08T03:00:00.000Z",
    key_id: "fd110d301d2f077d",
    received_signature: "ballot-box-signature",
}
const RECEIVED: IReceivedBallot = {
    tenant_id: "tenant-1",
    election_event_id: "event-1",
    election_id: "election-1",
    ballot_hash: BALLOT_HASH,
    voter_signing_pk: "voter-key",
    voter_ballot_signature: "voter-signature",
    ...ANSWER,
}

const ballotStyle = (policy?: EReceiptsPolicy, withKey = true): IBallotStyle =>
    ({
        id: "style-1",
        election_id: "election-1",
        election_event_id: "event-1",
        tenant_id: "tenant-1",
        ballot_eml: {
            election_event_presentation: policy ? {receipts: {policy}} : {},
            ballot_box_key: withKey ? BALLOT_BOX_KEY : undefined,
        },
    }) as IBallotStyle

const auditableBallot = (signed = true): IAuditableBallot =>
    ({
        ballot_hash: BALLOT_HASH,
        voter_signing_pk: signed ? "voter-key" : undefined,
        voter_ballot_signature: signed ? "voter-signature" : undefined,
    }) as IAuditableBallot

const receive = (
    overrides: Partial<Parameters<typeof useReceiveBallot>[0]> = {},
    policy: EReceiptsPolicy | null = EReceiptsPolicy.SIGNED_BY_BALLOT_BOX
) =>
    renderHook(() =>
        useReceiveBallot({
            ballotStyle: ballotStyle(policy ?? undefined),
            auditableBallot: auditableBallot(),
            ballotHash: BALLOT_HASH,
            isMultiContest: false,
            skip: false,
            ...overrides,
        })
    )

beforeEach(() => {
    jest.clearAllMocks()
    jest.spyOn(console, "log").mockImplementation(() => undefined)
    jest.spyOn(console, "error").mockImplementation(() => undefined)
    mockState = {receivedBallots: {}} as RootState
    mockReceiveBallot.mockResolvedValue({data: {receive_ballot: ANSWER}})
    mockVerifyReceivedBallot.mockReturnValue("FTBE-MHRX")
})
afterEach(() => jest.restoreAllMocks())

it.each([
    ["the event has no receipts setting", null, false],
    ["receipts are disabled", EReceiptsPolicy.DISABLED, false],
    ["the ballot is a demo or has nothing to cast", EReceiptsPolicy.SIGNED_BY_BALLOT_BOX, true],
])("sends nothing at review when %s", (_, policy, skip) => {
    const {result} = receive({skip}, policy)

    expect(result.current).toEqual({status: EReceiveBallotStatus.NOT_REQUIRED})
    expect(mockReceiveBallot).not.toHaveBeenCalled()
})

it("sends the voter-signed ballot and stores the receipt this device has checked", async () => {
    const {result} = receive()

    expect(result.current).toEqual({status: EReceiveBallotStatus.PENDING})
    await waitFor(() =>
        expect(mockDispatch).toHaveBeenCalledWith(
            setReceivedBallot({electionId: "election-1", receivedBallot: RECEIVED})
        )
    )
    expect(mockReceiveBallot).toHaveBeenCalledTimes(1)
    expect(mockReceiveBallot).toHaveBeenCalledWith({
        variables: {
            electionId: "election-1",
            ballotId: BALLOT_HASH,
            content: JSON.stringify({single: BALLOT_HASH}),
        },
    })
    expect(mockVerifyReceivedBallot).toHaveBeenCalledWith(BALLOT_BOX_KEY, RECEIVED)
})

it("checks the signature over what this device sent, not over what the answer says", async () => {
    mockReceiveBallot.mockResolvedValue({
        data: {
            receive_ballot: {
                ...ANSWER,
                ballot_hash: "another-ballot",
                election_id: "another-election",
                voter_signing_pk: "another-voter",
            },
        },
    })
    receive()

    await waitFor(() => expect(mockVerifyReceivedBallot).toHaveBeenCalledTimes(1))
    expect(mockVerifyReceivedBallot).toHaveBeenCalledWith(BALLOT_BOX_KEY, RECEIVED)
})

it("sends a multi-contest ballot in its own form", async () => {
    receive({isMultiContest: true})

    await waitFor(() => expect(mockReceiveBallot).toHaveBeenCalledTimes(1))
    expect(mockReceiveBallot.mock.calls[0][0].variables.content).toBe(
        JSON.stringify({multi: BALLOT_HASH})
    )
})

it("shows the Ballot ID of a ballot already received without sending it again", () => {
    mockState = {receivedBallots: {"election-1": RECEIVED}} as unknown as RootState
    const {result} = receive()

    expect(result.current).toEqual({
        status: EReceiveBallotStatus.RECEIVED,
        ballotId: "FTBE-MHRX",
    })
    expect(mockReceiveBallot).not.toHaveBeenCalled()
})

it("sends an edited ballot, whose earlier receipt names another ballot", async () => {
    mockState = {
        receivedBallots: {"election-1": {...RECEIVED, ballot_hash: "earlier-ballot"}},
    } as unknown as RootState
    const {result} = receive()

    expect(result.current).toEqual({status: EReceiveBallotStatus.PENDING})
    await waitFor(() => expect(mockReceiveBallot).toHaveBeenCalledTimes(1))
})

it("does not send the same ballot twice while it waits or after it fails", async () => {
    mockReceiveBallot.mockRejectedValue(new TypeError("Failed to fetch"))
    const {result, rerender} = receive()

    await waitFor(() => expect(result.current.status).toBe(EReceiveBallotStatus.FAILED))
    rerender()
    rerender()

    expect(mockReceiveBallot).toHaveBeenCalledTimes(1)
})

it("reports a network failure with the review screen's network text", async () => {
    mockReceiveBallot.mockRejectedValue(new TypeError("Failed to fetch"))
    const {result} = receive()

    await waitFor(() =>
        expect(result.current).toEqual({
            status: EReceiveBallotStatus.FAILED,
            errorMsg: "reviewScreen.error.NETWORK_ERROR",
        })
    )
    expect(mockDispatch).not.toHaveBeenCalled()
})

it("reports a refusal with the cast error text for its code", async () => {
    mockReceiveBallot.mockRejectedValue({
        message: "refused",
        graphQLErrors: [{extensions: {code: "CheckStatusFailed"}}],
    })
    const {result} = receive()

    await waitFor(() =>
        expect(result.current.errorMsg).toBe("reviewScreen.error.CAST_VOTE_CheckStatusFailed")
    )
})

it("reports an answer without a receipt as data that could not be fetched", async () => {
    mockReceiveBallot.mockResolvedValue({data: {receive_ballot: null}})
    const {result} = receive()

    await waitFor(() =>
        expect(result.current.errorMsg).toBe("reviewScreen.error.UNABLE_TO_FETCH_DATA")
    )
    expect(mockVerifyReceivedBallot).not.toHaveBeenCalled()
})

it("refuses a receipt the published ballot box key did not sign", async () => {
    mockVerifyReceivedBallot.mockImplementation(() => {
        throw new Error("The ballot box signature does not verify")
    })
    const {result} = receive()

    await waitFor(() =>
        expect(result.current).toEqual({
            status: EReceiveBallotStatus.FAILED,
            errorMsg: `reviewScreen.error.INCONSISTENT_HASH FTBE-MHRX ${BALLOT_HASH}`,
        })
    )
    expect(mockDispatch).not.toHaveBeenCalled()
})

it.each([
    [
        "the ballot style publishes no ballot box key",
        {ballotStyle: ballotStyle(EReceiptsPolicy.SIGNED_BY_BALLOT_BOX, false)},
    ],
    ["the ballot carries no voter signature", {auditableBallot: auditableBallot(false)}],
])("cannot receive a ballot when %s", async (_, overrides) => {
    const {result} = receive(overrides)

    await waitFor(() =>
        expect(result.current).toEqual({
            status: EReceiveBallotStatus.FAILED,
            errorMsg: "reviewScreen.error.CAST_VOTE",
        })
    )
    expect(mockReceiveBallot).not.toHaveBeenCalled()
})

it("waits for the ballot before sending anything", () => {
    const {result} = receive({auditableBallot: undefined, ballotHash: undefined})

    expect(result.current).toEqual({status: EReceiveBallotStatus.PENDING})
    expect(mockReceiveBallot).not.toHaveBeenCalled()
})
