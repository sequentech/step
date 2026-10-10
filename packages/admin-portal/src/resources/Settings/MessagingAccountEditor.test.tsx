/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EProviderApproval,
    EReadinessPolicy,
    IMessagingAccount,
} from "@/types/messaging"
import {MessagingAccountEditor} from "./MessagingAccountEditor"

const mockUpsert = jest.fn()
const mockReplace = jest.fn()
const mockNotify = jest.fn()
jest.mock("@apollo/client", () => ({
    gql: jest.fn(() => ({})),
    useMutation: (document: {name: string}) => [
        document.name === "upsert" ? mockUpsert : mockReplace,
    ],
}))
jest.mock("@/queries/UpsertMessagingAccount", () => ({UPSERT_MESSAGING_ACCOUNT: {name: "upsert"}}))
jest.mock("@/queries/ReplaceMessagingAccountCredentials", () => ({
    REPLACE_MESSAGING_ACCOUNT_CREDENTIALS: {name: "replace"},
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("react-admin", () => ({useNotify: () => mockNotify}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({ChannelLabel: ({label}: {label: string}) => <span>{label}</span>}),
    {virtual: true}
)

const messenger = (overrides: Partial<IMessagingAccount> = {}): IMessagingAccount => ({
    id: "account-1",
    tenant_id: "tenant-1",
    channel: EMessageChannel.MESSENGER,
    provider: EMessagingProvider.MESSENGER_SEND_API,
    name: "COMELEC Page",
    sender: {
        provider: EMessagingProvider.MESSENGER_SEND_API,
        page_id: "1234",
        page_name: "COMELEC Overseas Voting",
        page_username: "comelec",
        api_version: "v23.0",
        api_base_url: null,
    },
    credentials: {ACCESS_TOKEN: {replaced_at: "2026-09-30T10:00:00Z"}},
    limits: {allowed_calling_codes: []},
    provider_approval: EProviderApproval.PENDING,
    status: null,
    webhook_key: "hook-1",
    is_default: false,
    ...overrides,
})

const renderEditor = (account?: IMessagingAccount, canWrite = true) => {
    const onChanged = jest.fn()
    const onClose = jest.fn()
    render(
        <MessagingAccountEditor
            account={account}
            canWrite={canWrite}
            onChanged={onChanged}
            onClose={onClose}
        />
    )
    return {onChanged, onClose}
}

beforeEach(() => {
    mockUpsert.mockReset()
    mockReplace.mockReset()
    mockNotify.mockReset()
})

it("renders only the selected provider's identifiers", () => {
    renderEditor(messenger())
    expect(screen.getByLabelText(/messagingAccounts.field.page_id/)).toHaveValue("1234")
    expect(screen.getByLabelText(/messagingAccounts.field.page_username/)).toHaveValue("comelec")
    expect(screen.queryByLabelText(/messagingAccounts.field.from_address/)).toBeNull()
    expect(screen.queryByLabelText(/messagingAccounts.field.phone_number_id/)).toBeNull()
    expect(screen.getByLabelText("messagingAccounts.webhook.path")).toHaveValue(
        "/webhooks/meta/hook-1"
    )
})

it("starts a new account with the email provider's fields", () => {
    renderEditor()
    expect(screen.getByLabelText(/messagingAccounts.field.from_address/)).toBeInTheDocument()
    expect(
        screen.getByLabelText(/messagingAccounts.field.notification_topic_arn/)
    ).toBeInTheDocument()
    expect(screen.queryByLabelText(/messagingAccounts.field.page_id/)).toBeNull()
})

it("never shows a set credential, only when it was replaced", () => {
    renderEditor(messenger())
    expect(screen.getByText("messagingAccounts.credentials.set")).toBeInTheDocument()
    expect(screen.queryByLabelText("messaging.credential.ACCESS_TOKEN")).toBeNull()
    expect(screen.getByLabelText("messaging.credential.APP_SECRET")).toHaveAttribute(
        "type",
        "password"
    )
    expect(screen.queryByLabelText("messaging.credential.VERIFY_TOKEN")).toBeNull()
    fireEvent.click(
        screen.getByRole("button", {name: "messagingAccounts.credentials.replaceNamed"})
    )
    expect(screen.getByLabelText("messaging.credential.ACCESS_TOKEN")).toHaveValue("")
})

it("warns that voters must reconnect when the Page changes", () => {
    renderEditor(messenger())
    expect(screen.queryByText("messagingAccounts.warning.pageChange")).toBeNull()
    fireEvent.change(screen.getByLabelText(/messagingAccounts.field.page_id/), {
        target: {value: "9999"},
    })
    expect(screen.getByText("messagingAccounts.warning.pageChange")).toBeInTheDocument()
})

it("saves the account, then only the credentials that were typed", async () => {
    mockUpsert.mockResolvedValue({data: {upsert_messaging_account: {id: "account-1"}}})
    mockReplace.mockResolvedValue({data: {replace_messaging_account_credentials: {replaced: []}}})
    const {onChanged, onClose} = renderEditor(messenger())
    fireEvent.change(screen.getByLabelText("messaging.credential.APP_SECRET"), {
        target: {value: "s3cret"},
    })
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
    await waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(mockUpsert).toHaveBeenCalledWith({
        variables: {
            id: "account-1",
            channel: EMessageChannel.MESSENGER,
            name: "COMELEC Page",
            sender: messenger().sender,
            limits: {
                messages_per_second: null,
                otp_reserved_per_second: null,
                allowed_calling_codes: [],
            },
            providerApproval: null,
            readiness: EReadinessPolicy.PROVIDER_CHECK,
            isDefault: false,
        },
    })
    expect(mockReplace).toHaveBeenCalledWith({
        variables: {id: "account-1", credentials: {APP_SECRET: "s3cret"}},
    })
    expect(onChanged).toHaveBeenCalled()
})

it("saves a readiness confirmed by an administrator and a Graph API base URL", async () => {
    mockUpsert.mockResolvedValue({data: {upsert_messaging_account: {id: "account-1"}}})
    const {onClose} = renderEditor(
        messenger({readiness: EReadinessPolicy.ADMIN_CONFIRMED, status: null})
    )
    expect(screen.getByText("messagingAccounts.fieldHelp.readiness")).toBeInTheDocument()
    expect(screen.getByText("messaging.readinessPolicy.ADMIN_CONFIRMED")).toBeInTheDocument()
    fireEvent.change(screen.getByLabelText(/messagingAccounts.field.api_base_url/), {
        target: {value: "https://graph.bsp.example"},
    })
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
    await waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(mockUpsert.mock.calls[0][0].variables).toMatchObject({
        readiness: EReadinessPolicy.ADMIN_CONFIRMED,
        sender: {api_base_url: "https://graph.bsp.example"},
    })
})

it("leaves WhatsApp's provider approval to the administrator", () => {
    renderEditor(
        messenger({
            channel: EMessageChannel.WHATSAPP,
            provider: EMessagingProvider.WHATSAPP_CLOUD_API,
            sender: {
                provider: EMessagingProvider.WHATSAPP_CLOUD_API,
                business_account_id: "1",
                phone_number_id: "2",
                display_phone_number: "+63 917 000 0000",
                api_version: "v23.0",
            },
            provider_approval: EProviderApproval.CONFIRMED,
        })
    )
    expect(screen.getByText("messaging.approval.CONFIRMED")).toBeInTheDocument()
    expect(screen.getByLabelText("messagingAccounts.field.provider_approval")).not.toHaveAttribute(
        "aria-disabled"
    )
})

it("describes a custom HTTP API and shows its callback", () => {
    renderEditor(
        messenger({
            channel: EMessageChannel.VIBER,
            provider: EMessagingProvider.HTTP_API,
            sender: {
                provider: EMessagingProvider.HTTP_API,
                label: "COMELEC",
                send: {url: "https://partner.example/{{recipient}}"},
                template_required_for: [EMessagePurpose.OTP],
            },
            credentials: {},
        })
    )
    expect(screen.getByLabelText(/messagingAccounts.field.label/)).toHaveValue("COMELEC")
    expect(screen.getByText("messagingAccounts.http.section.SEND")).toBeInTheDocument()
    expect(screen.getByRole("alert")).toHaveTextContent(
        "messagingAccounts.http.problem.UNKNOWN_PLACEHOLDER"
    )
    expect(screen.getByLabelText("messaging.credential.WEBHOOK_SECRET")).toBeInTheDocument()
    expect(screen.getByLabelText("messagingAccounts.webhook.path")).toHaveValue(
        "/webhooks/http/hook-1"
    )
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
    expect(mockUpsert).not.toHaveBeenCalled()
})

it("does not save an invalid form and names the problem", () => {
    renderEditor(messenger())
    fireEvent.change(screen.getByLabelText(/messagingAccounts.field.page_id/), {
        target: {value: ""},
    })
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
    expect(screen.getByText("messagingAccounts.error.REQUIRED")).toBeInTheDocument()
    expect(mockUpsert).not.toHaveBeenCalled()
})

it("reports a failed save without closing", async () => {
    mockUpsert.mockRejectedValue(new Error("down"))
    const {onClose} = renderEditor(messenger())
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith("messagingAccounts.save.error", {type: "error"})
    )
    expect(onClose).not.toHaveBeenCalled()
})

it("is read-only without the write permission", () => {
    renderEditor(messenger(), false)
    expect(screen.getByLabelText(/messagingAccounts.field.page_id/)).toBeDisabled()
    expect(screen.queryByRole("button", {name: "messagingAccounts.editor.save"})).toBeNull()
    expect(screen.queryByRole("button", {name: "messagingAccounts.webhook.generate"})).toBeNull()
})
