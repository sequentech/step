/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {EMessageAttemptState, EMessageChannel, ILedgerMessage} from "@/types/messaging"
import {VoterMessagingSummary} from "./VoterMessagingSummary"

// The voter's rows the ledger holds; each query answers from them.
let mockRows: ILedgerMessage[] = []
jest.mock("@apollo/client", () => ({
    useQuery: (document: {name: string}, options: {skip?: boolean; variables?: any}) => {
        if (options.skip) return {data: undefined, loading: false}
        const rows =
            document.name === "attempts"
                ? mockRows.filter((row) => options.variables.keys.includes(row.logical_key))
                : mockRows
                      .filter((row) => ["FAILED", "UNKNOWN"].includes(String(row.state)))
                      .sort((a, b) => String(b.updated_at).localeCompare(String(a.updated_at)))
        return {data: {sequent_backend_message: rows}, loading: false}
    },
}))
jest.mock("@/queries/GetLedgerMessages", () => ({
    GET_VOTER_LAST_FAILURE: {name: "problems"},
    GET_VOTER_UNSETTLED_MESSAGES: {name: "problems"},
    GET_VOTER_MESSAGE_ATTEMPTS: {name: "attempts"},
}))
jest.mock("react-admin", () => ({useListContext: jest.fn(), useRecordContext: jest.fn()}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({ChannelLabel: ({label}: {label: string}) => <span>{label}</span>}),
    {virtual: true}
)
jest.mock("@/components/styles/ElectionHeaderStyles", () => ({
    ElectionHeaderStyles: {Title: ({children}: {children: React.ReactNode}) => <h3>{children}</h3>},
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-2"]}))
jest.mock("@/hooks/useMessagingReadRole", () => ({
    useMessagingReadRole: () => "messaging-config-read",
    roleContext: () => undefined,
}))
jest.mock("@/hooks/useMessagingAccounts", () => ({
    useMessagingAccounts: () => ({accounts: []}),
}))

const now = Date.now()
const minutesAgo = (minutes: number) => new Date(now - minutes * 60_000).toISOString()

const row = (overrides: Partial<ILedgerMessage>): ILedgerMessage => ({
    id: `id-${Math.random()}`,
    channel: EMessageChannel.WHATSAPP,
    state: EMessageAttemptState.FAILED,
    attempt: 1,
    error: null,
    logical_key: "bulk:send-1:voter-1",
    created_at: minutesAgo(30),
    updated_at: minutesAgo(30),
    ...overrides,
})

const renderSummary = () =>
    render(
        <VoterMessagingSummary
            attributes={{
                "sequent.read-only.message-channel": ["WHATSAPP"],
                "sequent.read-only.message-number": ["+34600111214"],
            }}
            userId="voter-1"
            electionEventId="event-2"
        />
    )

const rowText = (label: string) => screen.getByText(label).parentElement?.textContent ?? ""

describe("the voter's last failure (R4-07)", () => {
    it("never shows an unconfirmed message as a failure", () => {
        mockRows = [
            row({
                state: EMessageAttemptState.UNKNOWN,
                logical_key: "otp:1",
                updated_at: minutesAgo(10),
            }),
        ]
        renderSummary()
        expect(rowText("messaging.voter.lastFailure")).toContain("messaging.voter.noFailure")
        expect(rowText("messaging.voter.notConfirmed")).toContain("messaging.state.UNKNOWN")
    })

    it("drops a failure a later attempt of the same message delivered", () => {
        mockRows = [
            row({attempt: 1, state: EMessageAttemptState.FAILED, updated_at: minutesAgo(30)}),
            row({attempt: 2, state: EMessageAttemptState.DELIVERED, updated_at: minutesAgo(20)}),
        ]
        renderSummary()
        expect(rowText("messaging.voter.lastFailure")).toContain("messaging.voter.noFailure")
    })

    it("shows a failed message with its reason", () => {
        mockRows = [
            row({
                attempt: 3,
                state: EMessageAttemptState.FAILED,
                error: "Channel not offered for notices",
            }),
        ]
        renderSummary()
        expect(rowText("messaging.voter.lastFailure")).toContain("messaging.state.FAILED")
        expect(rowText("messaging.voter.lastFailure")).toContain(
            "messaging.reason.CHANNEL_NOT_OFFERED_FOR_NOTICES"
        )
    })

    it("shows a failure with no contact channel", () => {
        mockRows = [
            row({channel: null, error: "No contact channel", logical_key: "bulk:x:voter-1"}),
        ]
        renderSummary()
        expect(rowText("messaging.voter.lastFailure")).toContain(
            "messaging.reason.NO_CONTACT_CHANNEL"
        )
    })
})
