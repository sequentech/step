// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import {BallotLogProof} from "./BallotLogProof"

const mockVerify = jest.fn()
jest.mock("@sequentech/ui-core", () => ({
    verifyBallotLogProof: (...args: unknown[]) => mockVerify(...args),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: {count?: number}) =>
            options?.count === undefined ? key : `${key}:${options.count}`,
    }),
}))

const trust = {
    tenant_id: "tenant",
    election_event_id: "event",
    election_id: "election",
    system_public_key: "published-system-key",
    ballot_box_key: {key_id: "box-key-id", public_key: "published-box-key"},
}
const proof = {
    ballot_id: "FTBE-MHRX",
    pseudonym: "7QKM-2HXD",
    voting_channel: "ONLINE",
    received_entry: "signed-received-bytes",
    cast_entry: "signed-cast-bytes",
}
const passed = {binding: "passed", received: "passed", ballotId: "passed", cast: "passed"}

beforeEach(() => mockVerify.mockReset())
afterEach(() => {
    jest.restoreAllMocks()
    jest.useRealTimers()
})

test("all four checks must pass before verification succeeds; signed evidence remains inspectable", () => {
    mockVerify.mockReturnValue({checks: passed})
    render(<BallotLogProof trust={trust} proof={proof} />)
    expect(mockVerify).toHaveBeenCalledWith(trust, proof)
    expect(screen.getByRole("alert")).toHaveTextContent("ballotPrivacy.passed")
    expect(
        within(screen.getByRole("list")).getAllByText("ballotPrivacy.states.passed")
    ).toHaveLength(4)
    expect(screen.getByText(proof.pseudonym)).toBeVisible()
    expect(screen.getByText(trust.system_public_key)).toBeVisible()
    expect(screen.getByText(trust.ballot_box_key.key_id)).toBeVisible()
    fireEvent.click(screen.getByRole("button", {name: "ballotPrivacy.signedEntries:2"}))
    expect(screen.getByText(proof.received_entry)).toBeVisible()
    expect(screen.getByText(proof.cast_entry)).toBeVisible()
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test("unpublished cast evidence remains pending and allows retry without losing successful checks", () => {
    const retry = jest.fn()
    mockVerify.mockReturnValue({checks: {...passed, cast: "pending"}})
    const pendingProof = {...proof, cast_entry: null}
    const {rerender} = render(<BallotLogProof trust={trust} proof={pendingProof} onRetry={retry} />)
    expect(screen.getByRole("alert")).toHaveTextContent("ballotPrivacy.pendingHelp")
    expect(screen.getAllByText("ballotPrivacy.states.passed")).toHaveLength(3)
    expect(screen.getByRole("button", {name: "ballotPrivacy.signedEntries:1"})).toBeVisible()
    fireEvent.click(screen.getByRole("button", {name: "ballotPrivacy.retry"}))
    expect(retry).toHaveBeenCalledTimes(1)
    mockVerify.mockReturnValue({checks: passed})
    rerender(<BallotLogProof trust={trust} proof={proof} onRetry={retry} />)
    expect(screen.getByRole("alert")).toHaveTextContent("ballotPrivacy.verified")
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test("failed evidence takes priority over pending evidence and cannot be retried as unpublished", () => {
    mockVerify.mockReturnValue({checks: {...passed, binding: "failed", cast: "pending"}})
    render(<BallotLogProof trust={trust} proof={proof} onRetry={jest.fn()} />)
    expect(screen.getByRole("alert")).toHaveTextContent("ballotPrivacy.failedHelp")
    expect(screen.queryByText("ballotPrivacy.pendingHelp")).not.toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "ballotPrivacy.retry"})).not.toBeInTheDocument()
})

test.each(["throws", "missing report", "missing check"])("verifier %s fails closed", (failure) => {
    mockVerify.mockImplementation(() => {
        if (failure === "throws") throw new Error("Invalid signed bytes")
        return failure === "missing report" ? null : {checks: {...passed, binding: undefined}}
    })
    render(
        <BallotLogProof trust={trust} proof={{...proof, received_entry: null, cast_entry: null}} />
    )
    expect(screen.getByRole("alert")).toHaveTextContent("ballotPrivacy.failed")
    expect(screen.queryByText("ballotPrivacy.verified")).not.toBeInTheDocument()
    expect(screen.getByRole("button", {name: "ballotPrivacy.signedEntries:0"})).toBeVisible()
})

test("download preserves original scoped evidence and published trust, sanitizes the filename and releases the URL", async () => {
    jest.useFakeTimers()
    mockVerify.mockReturnValue({checks: passed})
    const create = jest.fn((_blob: Blob) => "blob:proof")
    const revoke = jest.fn()
    const originalCreate = Object.getOwnPropertyDescriptor(URL, "createObjectURL")
    const originalRevoke = Object.getOwnPropertyDescriptor(URL, "revokeObjectURL")
    Object.defineProperty(URL, "createObjectURL", {configurable: true, value: create})
    Object.defineProperty(URL, "revokeObjectURL", {configurable: true, value: revoke})
    const click = jest
        .spyOn(HTMLAnchorElement.prototype, "click")
        .mockImplementation(() => undefined)
    const downloadable = {...proof, ballot_id: "FTBE/MHRX?<unsafe>"}
    try {
        render(<BallotLogProof trust={trust} proof={downloadable} />)
        fireEvent.click(screen.getByRole("button", {name: "ballotPrivacy.download"}))
        const anchor = click.mock.contexts[0] as HTMLAnchorElement
        expect(anchor.download).toBe("ballot-FTBEMHRXunsafe-signed-entries.json")
        expect(anchor.href).toBe("blob:proof")
        const blob = create.mock.calls[0][0] as Blob
        expect(blob.type).toBe("application/json")
        const contents = await new Promise<string>((resolve, reject) => {
            const reader = new FileReader()
            reader.onload = () => resolve(reader.result as string)
            reader.onerror = reject
            reader.readAsText(blob)
        })
        expect(JSON.parse(contents)).toEqual({trust, proof: downloadable})
        jest.runOnlyPendingTimers()
        expect(revoke).toHaveBeenCalledWith("blob:proof")
    } finally {
        if (originalCreate) Object.defineProperty(URL, "createObjectURL", originalCreate)
        else Reflect.deleteProperty(URL, "createObjectURL")
        if (originalRevoke) Object.defineProperty(URL, "revokeObjectURL", originalRevoke)
        else Reflect.deleteProperty(URL, "revokeObjectURL")
    }
})
