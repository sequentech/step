/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {BallotEntryProof} from "./BallotEntryProof"
import {BALLOT_BOX_ENTRY} from "@/queries/BallotBoxLog"

jest.mock("@apollo/client", () => ({useQuery: (...args: unknown[]) => mockQuery(...args)}))
jest.mock("@/queries/BallotBoxLog", () => ({BALLOT_BOX_ENTRY: {}, BALLOT_BOX_KEYS: {}}))
jest.mock("@/providers/EventTimeZoneProvider", () => ({useMyTimeZone: () => "UTC"}))
jest.mock("@sequentech/ui-core", () => ({
    verifyBallotLogProof: (...args: unknown[]) => mockVerify(...args),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => key,
        i18n: {language: "en", resolvedLanguage: "en"},
    }),
}))

const mockQuery = jest.fn()
const mockVerify = jest.fn()
const entryRefetch = jest.fn().mockResolvedValue(undefined)
const keysRefetch = jest.fn().mockResolvedValue(undefined)
const proof = {ballot_id: "FTBE-MHRX", pseudonym: "7QKM-2HXD", voting_channel: "ONLINE"}
const trust = {election_id: "election", election_event_id: "event", ballot_box_key: {key_id: "key"}}
const props = {
    electionEventId: "event",
    electionId: "election",
    ballotId: "FTBE-MHRX",
    pseudonym: "7QKM-2HXD",
    selectedEntry: {id: 1, statement_kind: "BallotReceived"},
    zone: "UTC",
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

test("a verified ballot has no retry action", () => {
    render(<BallotEntryProof {...props} />)
    expect(screen.getByText("ballotPrivacy.passed")).toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test("pending evidence can be reloaded and retry disappears after verification succeeds", async () => {
    mockVerify.mockReturnValue({
        checks: {binding: "passed", received: "passed", ballotId: "passed", cast: "pending"},
    })
    const {rerender} = render(<BallotEntryProof {...props} />)
    await act(async () => {
        await userEvent.click(screen.getByRole("button", {name: "ballotPrivacy.retry"}))
    })
    expect(entryRefetch).toHaveBeenCalledTimes(1)
    expect(keysRefetch).toHaveBeenCalledTimes(1)
    mockVerify.mockReturnValue({
        checks: {binding: "passed", received: "passed", ballotId: "passed", cast: "passed"},
    })
    queries({data: {ballot_box_entry: {payload: {...proof}}}})
    rerender(<BallotEntryProof {...props} />)
    expect(screen.getByText("ballotPrivacy.passed")).toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test("failed verification keeps its warning without offering to retry", () => {
    mockVerify.mockReturnValue({
        checks: {binding: "failed", received: "pending", ballotId: "pending", cast: "pending"},
    })
    render(<BallotEntryProof {...props} />)
    expect(screen.getByText("ballotPrivacy.failedHelp")).toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test.each(["entry", "keys"])("loading %s shows progress without a retry action", (loading) => {
    queries(loading === "entry" ? {loading: true} : {}, loading === "keys" ? {loading: true} : {})
    render(<BallotEntryProof {...props} />)
    expect(screen.getByRole("progressbar")).toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test.each([
    [{error: new Error("Evidence unavailable")}, {}],
    [{}, {error: new Error("Keys unavailable")}],
    [{data: undefined}, {}],
    [{}, {data: undefined}],
    [{data: {ballot_box_entry: {payload: {...proof, ballot_id: "OTHER-ID"}}}}, {}],
])("proof load errors offer a retry of both queries (%#)", async (entry, keys) => {
    queries(entry, keys)
    render(<BallotEntryProof {...props} />)
    expect(screen.getByText("ballotPrivacy.proofError")).toBeInTheDocument()
    await act(async () => {
        await userEvent.click(screen.getByRole("button", {name: "ballotPrivacy.retry"}))
    })
    expect(entryRefetch).toHaveBeenCalledTimes(1)
    expect(keysRefetch).toHaveBeenCalledTimes(1)
})
