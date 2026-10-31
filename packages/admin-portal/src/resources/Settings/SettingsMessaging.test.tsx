/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {
    EMessageChannel,
    EMessagingProvider,
    EProviderApproval,
    IMessagingAccount,
} from "@/types/messaging"
import {SettingsMessaging} from "./SettingsMessaging"

let mockAccounts: IMessagingAccount[] = []
const mockRefetch = jest.fn()

jest.mock("@apollo/client", () => ({
    gql: jest.fn(() => ({})),
    useMutation: () => [jest.fn()],
}))
jest.mock("@/queries/CheckMessagingAccount", () => ({CHECK_MESSAGING_ACCOUNT: {}}))
jest.mock("@/queries/DeleteMessagingAccount", () => ({DELETE_MESSAGING_ACCOUNT: {}}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("react-admin", () => ({
    useNotify: () => jest.fn(),
    useGetOne: () => ({data: undefined}),
}))
jest.mock("@/providers/AuthContextProvider", () => {
    const {createContext} = jest.requireActual("react")
    return {AuthContext: createContext({isAuthorized: () => true})}
})
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-2"]}))
jest.mock("@/hooks/useMessagingAccounts", () => ({
    useMessagingAccounts: () => ({
        accounts: mockAccounts,
        loading: false,
        error: undefined,
        refetch: mockRefetch,
    }),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        ChannelLabel: ({label}: {label: string}) => <span>{label}</span>,
        Dialog: () => null,
    }),
    {virtual: true}
)
jest.mock("./MessagingTestDialog", () => ({MessagingTestDialog: () => null}))
jest.mock("./MessagingMonthlyVolumes", () => ({MessagingMonthlyVolumes: () => null}))
// The Sent messages tab has its own tests (SentMessages/); here, only the accounts.
jest.mock("./SentMessages/MessagingTabs", () => ({
    MessagingTabs: ({accounts}: {accounts: React.ReactNode}) => accounts,
}))
// The drawer's editor, reduced to what it was given.
jest.mock("./MessagingAccountEditor", () => ({
    MESSAGING_ACCOUNT_EDITOR_TITLE: "editor-title",
    formatDate: (value: string) => value,
    MessagingAccountEditor: ({account}: {account?: IMessagingAccount}) => (
        <div data-testid="editor">
            {account?.credentials?.REPORT_PASSWORD ? "report credentials set" : "not generated"}
        </div>
    ),
}))

const viber = (overrides: Partial<IMessagingAccount> = {}): IMessagingAccount => ({
    id: "account-v",
    tenant_id: "tenant-2",
    channel: EMessageChannel.VIBER,
    provider: EMessagingProvider.VIBER_INFOBIP,
    name: "Staff Association Viber",
    sender: {
        provider: EMessagingProvider.VIBER_INFOBIP,
        base_url: "https://x.api.infobip.com",
        sender: "Staff Association",
    },
    credentials: {API_KEY: {replaced_at: "2026-09-30T10:00:00Z"}},
    limits: {allowed_calling_codes: []},
    provider_approval: EProviderApproval.PENDING,
    status: null,
    is_default: true,
    ...overrides,
})

it("shows the account as the list reads it again, not as it was when the drawer opened (R4-09)", () => {
    mockAccounts = [viber()]
    const {rerender} = render(<SettingsMessaging />)
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.action.editNamed"}))
    expect(screen.getByTestId("editor")).toHaveTextContent("not generated")
    // Report credentials were generated; the list was read again.
    mockAccounts = [
        viber({
            credentials: {
                API_KEY: {replaced_at: "2026-09-30T10:00:00Z"},
                REPORT_USERNAME: {replaced_at: "2026-10-09T10:00:00Z"},
                REPORT_PASSWORD: {replaced_at: "2026-10-09T10:00:00Z"},
            },
        }),
    ]
    rerender(<SettingsMessaging />)
    expect(screen.getByTestId("editor")).toHaveTextContent("report credentials set")
})
