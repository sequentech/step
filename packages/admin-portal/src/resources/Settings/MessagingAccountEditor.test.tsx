/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {
    EMessageChannel,
    EMessagingProvider,
    EProviderApproval,
    EReadinessPolicy,
    IMessagingAccount,
} from "@/types/messaging"
import {MessagingAccountEditor} from "./MessagingAccountEditor"

const CALLBACK_URL = "https://callbacks.example.org/webhooks/meta/" + "a".repeat(64)

const mockUpsert = jest.fn()
const mockReplace = jest.fn()
const mockNotify = jest.fn()
const mockCallback = jest.fn()
jest.mock("@apollo/client", () => ({
    gql: jest.fn(() => ({})),
    useMutation: (document: {name: string}) => [
        document.name === "upsert" ? mockUpsert : mockReplace,
    ],
    useQuery: (_document: unknown, options: {skip?: boolean}) =>
        options.skip ? {data: undefined, loading: false} : mockCallback(options),
}))
jest.mock("@/queries/UpsertMessagingAccount", () => ({UPSERT_MESSAGING_ACCOUNT: {name: "upsert"}}))
jest.mock("@/queries/ReplaceMessagingAccountCredentials", () => ({
    REPLACE_MESSAGING_ACCOUNT_CREDENTIALS: {name: "replace"},
}))
jest.mock("@/queries/MessagingAccountCallbackUrl", () => ({
    MESSAGING_ACCOUNT_CALLBACK_URL: {name: "callback"},
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
    name: "Staff Association Page",
    sender: {
        provider: EMessagingProvider.MESSENGER_SEND_API,
        page_id: "1234",
        page_name: "Staff Association",
        page_username: "staffassociation",
        api_version: "v23.0",
        api_base_url: null,
    },
    credentials: {ACCESS_TOKEN: {replaced_at: "2026-09-30T10:00:00Z"}},
    limits: {allowed_calling_codes: []},
    provider_approval: EProviderApproval.PENDING,
    status: null,
    is_default: false,
    ...overrides,
})

const renderEditor = (account?: IMessagingAccount, canWrite = true) => {
    const onChanged = jest.fn()
    const onClose = jest.fn()
    const onCreated = jest.fn()
    render(
        <MessagingAccountEditor
            account={account}
            canWrite={canWrite}
            onChanged={onChanged}
            onCreated={onCreated}
            onClose={onClose}
        />
    )
    return {onChanged, onClose, onCreated}
}

beforeEach(() => {
    mockUpsert.mockReset()
    mockReplace.mockReset()
    mockNotify.mockReset()
    mockCallback.mockReset()
    mockCallback.mockReturnValue({
        data: {messaging_account_callback_url: {callback_url: CALLBACK_URL, reason: null}},
        loading: false,
    })
})

it("renders only the selected provider's identifiers", () => {
    renderEditor(messenger())
    expect(screen.getByLabelText(/messagingAccounts.field.page_id/)).toHaveValue("1234")
    expect(screen.getByLabelText(/messagingAccounts.field.page_username/)).toHaveValue(
        "staffassociation"
    )
    expect(screen.queryByLabelText(/messagingAccounts.field.from_address/)).toBeNull()
    expect(screen.queryByLabelText(/messagingAccounts.field.phone_number_id/)).toBeNull()
})

it("shows the full callback URL the backend gives, and Messenger's own description", () => {
    renderEditor(messenger())
    expect(screen.getByLabelText("messagingAccounts.webhook.url")).toHaveValue(CALLBACK_URL)
    expect(mockCallback).toHaveBeenCalledWith(
        expect.objectContaining({variables: {accountId: "account-1"}})
    )
    expect(
        screen.getByText("messagingAccounts.webhook.description.MESSENGER_SEND_API")
    ).toBeInTheDocument()
    expect(screen.getByRole("button", {name: "messagingAccounts.webhook.generate"})).toBeVisible()
})

it("shows the backend's reason when there is no callback URL", () => {
    mockCallback.mockReturnValue({
        data: {
            messaging_account_callback_url: {
                callback_url: null,
                reason: "Callback address not configured",
            },
        },
        loading: false,
    })
    renderEditor(messenger())
    expect(screen.getByLabelText("messagingAccounts.webhook.url")).toHaveValue(
        "messaging.reason.CALLBACK_NOT_CONFIGURED"
    )
})

it("starts a new account with the WhatsApp provider's fields, callback URL after saving", () => {
    renderEditor()
    expect(screen.getByLabelText(/messagingAccounts.field.phone_number_id/)).toBeInTheDocument()
    expect(screen.queryByLabelText(/messagingAccounts.field.page_id/)).toBeNull()
    expect(screen.getByLabelText("messagingAccounts.webhook.url")).toHaveValue(
        "messagingAccounts.webhook.afterSaving"
    )
    expect(mockCallback).not.toHaveBeenCalled()
})

it("generates Viber's report credentials and shows them once with the steps", async () => {
    mockReplace.mockResolvedValue({
        data: {
            replace_messaging_account_credentials: {
                replaced: ["REPORT_USERNAME", "REPORT_PASSWORD"],
                verify_token: null,
                report_username: "reports-1",
                report_password: "generated-secret",
            },
        },
    })
    const {onChanged} = renderEditor(
        messenger({
            channel: EMessageChannel.VIBER,
            provider: EMessagingProvider.VIBER_INFOBIP,
            sender: {
                provider: EMessagingProvider.VIBER_INFOBIP,
                base_url: "https://x.api.infobip.com",
                sender: "Staff Association",
            },
            credentials: {API_KEY: {replaced_at: "2026-09-30T10:00:00Z"}},
        })
    )
    expect(screen.queryByLabelText("messaging.credential.REPORT_PASSWORD")).toBeNull()
    expect(screen.queryByText(/APP_SECRET/)).toBeNull()
    expect(screen.getAllByText("messagingAccounts.reports.step.auth")).toHaveLength(1)
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.reports.generate"}))
    await waitFor(() =>
        expect(screen.getByLabelText("messaging.credential.REPORT_PASSWORD")).toHaveValue(
            "generated-secret"
        )
    )
    expect(mockReplace).toHaveBeenCalledWith({
        variables: {
            id: "account-1",
            credentials: {},
            generateVerifyToken: false,
            generateReportCredentials: true,
        },
    })
    expect(screen.getByLabelText("messaging.credential.REPORT_USERNAME")).toHaveValue("reports-1")
    expect(screen.getAllByText("messagingAccounts.reports.step.auth")).toHaveLength(2)
    expect(onChanged).toHaveBeenCalled()
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
            name: "Staff Association Page",
            sender: messenger().sender,
            limits: {
                messages_per_second: null,
                otp_reserved_per_second: null,
                allowed_calling_codes: [],
            },
            providerApproval: null,
            isDefault: false,
        },
    })
    expect(mockReplace).toHaveBeenCalledWith({
        variables: {id: "account-1", credentials: {APP_SECRET: "s3cret"}},
    })
    expect(onChanged).toHaveBeenCalled()
})

it("offers no readiness choice, and saves a Graph API base URL", async () => {
    mockUpsert.mockResolvedValue({data: {upsert_messaging_account: {id: "account-1"}}})
    const {onClose} = renderEditor(
        messenger({readiness: EReadinessPolicy.ADMIN_CONFIRMED, status: null})
    )
    expect(screen.queryByLabelText(/messagingAccounts.field.readiness/)).toBeNull()
    fireEvent.change(screen.getByLabelText(/messagingAccounts.field.api_base_url/), {
        target: {value: "https://graph.bsp.example"},
    })
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
    await waitFor(() => expect(onClose).toHaveBeenCalled())
    const variables = mockUpsert.mock.calls[0][0].variables
    expect(variables).toMatchObject({sender: {api_base_url: "https://graph.bsp.example"}})
    expect("readiness" in variables).toBe(false)
})

it("keeps a new account with a callback open to show its callback URL", async () => {
    mockUpsert.mockResolvedValue({
        data: {
            upsert_messaging_account: {
                id: "account-9",
                callback_url: CALLBACK_URL,
                callback_reason: null,
            },
        },
    })
    const {onClose, onCreated} = renderEditor()
    for (const [field, value] of [
        ["messagingAccounts.field.name", "Staff Association WhatsApp"],
        ["messagingAccounts.field.business_account_id", "1"],
        ["messagingAccounts.field.phone_number_id", "2"],
        ["messagingAccounts.field.display_phone_number", "+34 600 000 214"],
    ]) {
        fireEvent.change(screen.getByLabelText(new RegExp(field)), {target: {value}})
    }
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
    await waitFor(() =>
        expect(onCreated).toHaveBeenCalledWith("account-9", {
            callback_url: CALLBACK_URL,
            reason: null,
        })
    )
    expect(onClose).not.toHaveBeenCalled()
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

it("shows an account of a removed provider as unsupported, to be replaced, not saved", () => {
    renderEditor(
        messenger({
            channel: EMessageChannel.VIBER,
            provider: EMessagingProvider.HTTP_API,
            sender: {provider: EMessagingProvider.HTTP_API, label: "Old partner"},
            credentials: {},
        })
    )
    expect(screen.getByRole("alert")).toHaveTextContent("messagingAccounts.unsupported")
    expect(screen.queryByRole("button", {name: "messagingAccounts.editor.save"})).toBeNull()
    expect(screen.queryByLabelText("messagingAccounts.webhook.url")).toBeNull()
    expect(screen.queryByText(/messagingAccounts.http/)).toBeNull()
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
    // The callback URL action needs messaging-account-write: not read, and the reason is said.
    expect(mockCallback).not.toHaveBeenCalled()
    expect(screen.getByLabelText("messagingAccounts.webhook.url")).toHaveValue(
        "messagingAccounts.webhook.needsWrite"
    )
})
