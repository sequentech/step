/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {LedgerStatusField} from "./LedgerStatus"

let mockRecord: {message?: string} = {}
jest.mock("react-admin", () => ({
    useRecordContext: () => mockRecord,
    useListContext: () => ({data: []}),
}))
jest.mock("@apollo/client", () => ({gql: jest.fn(() => ({})), useQuery: () => ({})}))
jest.mock("@/queries/GetLedgerMessages", () => ({GET_LEDGER_MESSAGE_STATES: {}}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-2"]}))
jest.mock("@/hooks/useMessagingReadRole", () => ({
    useMessagingReadRole: () => null,
    roleContext: () => undefined,
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

const entry = (details: Record<string, unknown>) => ({
    message: JSON.stringify({
        statement: {body: {SendCommunications: JSON.stringify({channel: "sms", ...details})}},
    }),
})

it("shows a failed entry's reason, both parts translated (Follow-up 2)", () => {
    mockRecord = entry({
        outcome: "FAILED",
        reason: "Invalid destination for the channel; Can't be retried: no stored template",
    })
    render(<LedgerStatusField />)
    expect(screen.getByText("messaging.state.FAILED")).toBeInTheDocument()
    expect(
        screen.getByText(
            "messaging.reason.INVALID_DESTINATION; messaging.reason.NOT_RETRYABLE_NO_TEMPLATE"
        )
    ).toBeInTheDocument()
})
