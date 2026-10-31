// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {BallotBoxEntry} from "./BallotBoxEntry"
import {BALLOT_BOX_ENTRY} from "../queries/BallotBoxLog"

jest.mock("@apollo/client/react", () => ({useQuery: (...args: unknown[]) => mockQuery(...args)}))
jest.mock("../queries/BallotBoxLog", () => ({BALLOT_BOX_ENTRY: {}, BALLOT_BOX_KEYS: {}}))
jest.mock("@sequentech/ui-essentials", () => ({
    BallotLogProof: jest.requireActual(
        "../../../ui-essentials/src/components/BallotLogProof/BallotLogProof"
    ).BallotLogProof,
}))
jest.mock("@sequentech/ui-core", () => ({
    verifyBallotLogProof: (...args: unknown[]) => mockVerify(...args),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

const mockQuery = jest.fn()
const mockVerify = jest.fn()
const entryRefetch = jest.fn().mockResolvedValue(undefined)
const keysRefetch = jest.fn().mockResolvedValue(undefined)
const TRACKING_ID = "FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89AB"
const proof = {
    ballot_id: "QR7T-K2NM",
    tracking_id: TRACKING_ID,
    pseudonym: "K7QM-2XWD",
    voting_channel: "ONLINE",
}
const trust = {election_id: "election", election_event_id: "event", ballot_box_key: {key_id: "key"}}
const props = {
    electionEventId: "event",
    electionId: "election",
    ballotId: TRACKING_ID,
    onClose: jest.fn(),
}

const queries = (entry = {}, keys = {}) => {
    mockQuery.mockImplementation((query) =>
        query === BALLOT_BOX_ENTRY
            ? {data: {ballot_box_entry: {payload: proof}}, refetch: entryRefetch, ...entry}
            : {data: {ballot_box_keys: {payload: trust}}, refetch: keysRefetch, ...keys}
    )
}

beforeEach(() => {
    jest.clearAllMocks()
    queries()
    mockVerify.mockReturnValue({
        checks: {binding: "passed", received: "passed", ballotId: "passed", cast: "passed"},
    })
})

test("the dialog is named by the short Tracking ID, not the full one or the receipt key", () => {
    render(<BallotBoxEntry {...props} />)
    expect(screen.getByRole("dialog", {name: /ballotPrivacy.detailTitle/})).toBeInTheDocument()
    expect(mockQuery).toHaveBeenCalledWith(
        BALLOT_BOX_ENTRY,
        expect.objectContaining({variables: expect.objectContaining({ballotId: TRACKING_ID})})
    )
})

test("a ballot only received or audited opens its proof", () => {
    mockVerify.mockReturnValue({
        checks: {binding: "passed", received: "passed", ballotId: "passed", cast: "pending"},
    })
    queries({data: {ballot_box_entry: {payload: {...proof, stage: "received"}}}})
    render(<BallotBoxEntry {...props} />)
    expect(screen.getByText("ballotPrivacy.stages.received.title")).toBeInTheDocument()
})

test("a verified ballot has no retry action", () => {
    render(<BallotBoxEntry {...props} />)
    expect(screen.getByText("ballotPrivacy.passed")).toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test("pending evidence can be reloaded and retry disappears after verification succeeds", async () => {
    mockVerify.mockReturnValue({
        checks: {binding: "passed", received: "passed", ballotId: "passed", cast: "pending"},
    })
    const {rerender} = render(<BallotBoxEntry {...props} />)
    await userEvent.click(screen.getByRole("button", {name: "ballotPrivacy.retry"}))
    expect(entryRefetch).toHaveBeenCalledTimes(1)
    expect(keysRefetch).toHaveBeenCalledTimes(1)
    mockVerify.mockReturnValue({
        checks: {binding: "passed", received: "passed", ballotId: "passed", cast: "passed"},
    })
    queries({data: {ballot_box_entry: {payload: {...proof}}}})
    rerender(<BallotBoxEntry {...props} />)
    expect(screen.getByText("ballotPrivacy.passed")).toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test("failed verification keeps its warning without offering to retry", () => {
    mockVerify.mockReturnValue({
        checks: {binding: "failed", received: "pending", ballotId: "pending", cast: "pending"},
    })
    render(<BallotBoxEntry {...props} />)
    expect(screen.getByText("ballotPrivacy.failedHelp")).toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test.each(["entry", "keys"])("loading %s shows progress without a retry action", (loading) => {
    queries(loading === "entry" ? {loading: true} : {}, loading === "keys" ? {loading: true} : {})
    render(<BallotBoxEntry {...props} />)
    expect(screen.getByRole("progressbar")).toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test.each([
    [{error: new Error("Evidence unavailable")}, {}],
    [{}, {error: new Error("Keys unavailable")}],
    [{data: undefined}, {}],
    [{}, {data: undefined}],
    [{data: {ballot_box_entry: {payload: {...proof, tracking_id: "OTHER-ID"}}}}, {}],
])("proof load errors offer a retry of both queries (%#)", async (entry, keys) => {
    queries(entry, keys)
    render(<BallotBoxEntry {...props} />)
    expect(screen.getByText("ballotPrivacy.proofError")).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", {name: "ballotPrivacy.retry"}))
    expect(entryRefetch).toHaveBeenCalledTimes(1)
    expect(keysRefetch).toHaveBeenCalledTimes(1)
})
