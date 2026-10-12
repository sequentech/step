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
    IAccountCheck,
    IAccountCheckDetails,
    IMessagingAccount,
} from "@/types/messaging"
import {MessagingAccountEditor} from "./MessagingAccountEditor"

const CALLBACK_URL = "https://callbacks.example.org/webhooks/meta/" + "a".repeat(64)

const mockUpsert = jest.fn()
const mockReplace = jest.fn()
const mockNotify = jest.fn()
const mockCallback = jest.fn()
const mockImpact = jest.fn()
jest.mock("@apollo/client", () => ({
    gql: jest.fn(() => ({})),
    useMutation: (document: {name: string}) => [
        document.name === "upsert" ? mockUpsert : mockReplace,
    ],
    useQuery: (document: {name: string}, options: {skip?: boolean}) =>
        options.skip
            ? {data: undefined, loading: false}
            : document.name === "impact"
              ? mockImpact(options)
              : mockCallback(options),
}))
jest.mock("@/queries/UpsertMessagingAccount", () => ({UPSERT_MESSAGING_ACCOUNT: {name: "upsert"}}))
jest.mock("@/queries/ReplaceMessagingAccountCredentials", () => ({
    REPLACE_MESSAGING_ACCOUNT_CREDENTIALS: {name: "replace"},
}))
jest.mock("@/queries/MessagingAccountCallbackUrl", () => ({
    MESSAGING_ACCOUNT_CALLBACK_URL: {name: "callback"},
}))
jest.mock("@/queries/MessagingAccountChangeImpact", () => ({
    MESSAGING_ACCOUNT_CHANGE_IMPACT: {name: "impact"},
}))
jest.mock("react-i18next", () => ({
    // Keys only; a reason given to a text is appended, so tests can read it.
    useTranslation: () => ({
        t: (key: string, options?: {reason?: string; count?: number}) =>
            options?.reason
                ? `${key}: ${options.reason}`
                : options?.count !== undefined
                  ? `${key}: ${options.count}`
                  : key,
        i18n: {language: "en"},
    }),
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

const viber = (overrides: Partial<IMessagingAccount> = {}): IMessagingAccount =>
    messenger({
        channel: EMessageChannel.VIBER,
        provider: EMessagingProvider.VIBER_INFOBIP,
        name: "Staff Association Viber",
        sender: {
            provider: EMessagingProvider.VIBER_INFOBIP,
            base_url: "https://x.api.infobip.com",
            sender: "Staff Association",
        },
        credentials: {API_KEY: {replaced_at: "2026-09-30T10:00:00Z"}},
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
    mockImpact.mockReset()
    mockImpact.mockReturnValue({data: undefined, loading: true})
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
    expect(screen.getByText(/messagingAccounts.warning.pageChange /)).toBeInTheDocument()
    expect(screen.getByText(/messagingAccounts.warning.pageChangeCounting/)).toBeInTheDocument()
    expect(screen.getByText(/messagingAccounts.warning.recheck/)).toBeInTheDocument()
})

it("counts the voters who must reconnect when the Page changes (BC-F13)", () => {
    mockImpact.mockReturnValue({
        data: {messaging_account_change_impact: {affected_voters: 42, events: 2}},
        loading: false,
    })
    renderEditor(messenger())
    expect(mockImpact).not.toHaveBeenCalled()
    fireEvent.change(screen.getByLabelText(/messagingAccounts.field.page_id/), {
        target: {value: "9999"},
    })
    expect(mockImpact).toHaveBeenCalledWith(
        expect.objectContaining({variables: {accountId: "account-1"}})
    )
    expect(screen.getByText(/messagingAccounts.warning.pageChangeCount: 42/)).toBeInTheDocument()
})

it("says the count could not be read when the impact query fails", () => {
    mockImpact.mockReturnValue({data: undefined, loading: false, error: new Error("down")})
    renderEditor(messenger())
    fireEvent.change(screen.getByLabelText(/messagingAccounts.field.page_id/), {
        target: {value: "9999"},
    })
    expect(screen.getByText(/messagingAccounts.warning.pageChangeCountError/)).toBeInTheDocument()
})

it("saves the account with only the credentials that were typed, in one call", async () => {
    mockUpsert.mockResolvedValue({data: {upsert_messaging_account: {id: "account-1"}}})
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
                otp_concurrency: null,
                allowed_calling_codes: [],
            },
            providerApproval: null,
            isDefault: false,
            credentials: {APP_SECRET: "s3cret"},
        },
    })
    expect(mockReplace).not.toHaveBeenCalled()
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
    // A new address takes the access token again (it would be sent there).
    fireEvent.change(screen.getByLabelText("messaging.credential.ACCESS_TOKEN"), {
        target: {value: "page-token"},
    })
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
    await waitFor(() => expect(onClose).toHaveBeenCalled())
    const variables = mockUpsert.mock.calls[0][0].variables
    expect(variables).toMatchObject({
        sender: {api_base_url: "https://graph.bsp.example"},
        credentials: {ACCESS_TOKEN: "page-token"},
    })
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
    expect(screen.getAllByText("messaging.approval.CONFIRMED").length).toBeGreaterThan(0)
    expect(screen.getByLabelText("messagingAccounts.field.provider_approval")).not.toHaveAttribute(
        "aria-disabled"
    )
    expect(screen.getByText(/messagingAccounts.status.approvalEntered\b/)).toBeInTheDocument()
})

const whatsapp = (overrides: Partial<IMessagingAccount> = {}): IMessagingAccount =>
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
        provider_approval: EProviderApproval.PENDING,
        ...overrides,
    })

const checked = (details: IAccountCheckDetails): IAccountCheck => ({
    connected: true,
    production_access: true,
    approved_templates: {},
    checked_at: "2026-10-09T08:00:00Z",
    reason: null,
    details,
})

it("uses the approval the provider's check reports, with no choice to make", () => {
    renderEditor(
        whatsapp({
            status: checked({
                whatsapp_number: {
                    status: "CONNECTED",
                    name_status: "APPROVED",
                    quality_rating: "GREEN",
                    messaging_limit: "TIER_1K",
                },
                provider_approval: EProviderApproval.CONFIRMED,
                templates: [
                    {
                        name: "otp_code",
                        language: "en",
                        category: "AUTHENTICATION",
                        status: "APPROVED",
                    },
                ],
            }),
        })
    )
    expect(screen.queryByLabelText("messagingAccounts.field.provider_approval")).toBeNull()
    expect(screen.getByText("messaging.approval.CONFIRMED")).toBeInTheDocument()
    expect(screen.getByText(/messagingAccounts.status.approvalFromCheck/)).toBeInTheDocument()
    expect(screen.getByText("messagingAccounts.status.quality.GREEN")).toBeInTheDocument()
    expect(screen.getByText("TIER_1K")).toBeInTheDocument()
    expect(screen.getByText("otp_code")).toBeInTheDocument()
    expect(
        screen.getByText("messagingAccounts.status.templateCategory.AUTHENTICATION")
    ).toBeInTheDocument()
    expect(screen.getByText("messagingAccounts.status.templateStatus.APPROVED")).toBeInTheDocument()
})

it("shows an account whose last check was of another number as unchecked (R5-09)", () => {
    renderEditor(
        whatsapp({
            status: checked({
                identity: {
                    provider: EMessagingProvider.WHATSAPP_CLOUD_API,
                    ids: {business_account_id: "1", phone_number_id: "earlier"},
                },
            }),
        })
    )
    expect(screen.getByText("messagingAccounts.status.unchecked")).toBeInTheDocument()
    expect(screen.queryByText("messagingAccounts.status.qualityRating")).toBeNull()
})

it("shows a check of the current number, and a quality flag as a warning only (R5-17)", () => {
    renderEditor(
        whatsapp({
            status: checked({
                identity: {
                    provider: EMessagingProvider.WHATSAPP_CLOUD_API,
                    ids: {business_account_id: "1", phone_number_id: "2"},
                },
                whatsapp_number: {status: "FLAGGED", name_status: "APPROVED"},
                app_secret: {valid: false, reason: "Meta refused the app secret"},
            }),
        })
    )
    expect(screen.queryByText("messagingAccounts.status.unchecked")).toBeNull()
    expect(screen.getByText(/messagingAccounts.status.numberQualityWarning/)).toBeInTheDocument()
    expect(screen.getByText("messagingAccounts.status.appSecret")).toBeInTheDocument()
    expect(screen.getByText("messaging.reason.APP_SECRET_REFUSED")).toBeInTheDocument()
})

it("lists WhatsApp's number changes and the voters to ask, changing nothing", () => {
    renderEditor(
        whatsapp({
            status: checked({
                number_changes: [
                    {
                        masked_old_number: "+63*******000",
                        masked_new_number: "+63*******111",
                        voter_ids: ["voter-1"],
                        reported_at: "2026-10-09T09:00:00Z",
                    },
                ],
            }),
        })
    )
    expect(screen.getByText("messagingAccounts.status.numberChanges.title")).toBeInTheDocument()
    expect(screen.getByText(/messagingAccounts.status.numberChanges.voters/)).toBeInTheDocument()
})

it("shows the Page's permission, subscription and Get Started button", () => {
    renderEditor(
        messenger({
            status: checked({
                messenger: {pages_messaging: true, subscribed: true, get_started: false},
                token: {valid: true, expires_at: null, missing_scopes: []},
            }),
        })
    )
    expect(screen.getByText("messagingAccounts.status.getStartedMissing")).toBeInTheDocument()
    expect(screen.getByText("messagingAccounts.status.tokenNoExpiry")).toBeInTheDocument()
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

describe("an address change (R4-01)", () => {
    it("asks for the credentials again and saves them with the change, in one call", async () => {
        mockUpsert.mockResolvedValue({data: {upsert_messaging_account: {id: "account-1"}}})
        const {onClose} = renderEditor(viber())
        fireEvent.change(screen.getByLabelText(/messagingAccounts.field.base_url/), {
            target: {value: "https://y.api.infobip.com"},
        })
        // The stored API key would go to the new address: its input opens.
        fireEvent.change(screen.getByLabelText("messaging.credential.API_KEY"), {
            target: {value: "new-key"},
        })
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
        await waitFor(() => expect(onClose).toHaveBeenCalled())
        expect(mockUpsert).toHaveBeenCalledTimes(1)
        expect(mockUpsert.mock.calls[0][0].variables).toMatchObject({
            id: "account-1",
            sender: {base_url: "https://y.api.infobip.com"},
            credentials: {API_KEY: "new-key"},
        })
        expect(mockReplace).not.toHaveBeenCalled()
    })

    it("does not send an address change without the credentials, and says why", () => {
        renderEditor(viber())
        fireEvent.change(screen.getByLabelText(/messagingAccounts.field.base_url/), {
            target: {value: "https://y.api.infobip.com"},
        })
        expect(
            screen.getByText("messagingAccounts.error.CREDENTIALS_FOLLOW_ENDPOINT")
        ).toBeInTheDocument()
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
        expect(mockUpsert).not.toHaveBeenCalled()
    })

    it("shows the backend's reason when the save is refused", async () => {
        mockUpsert.mockRejectedValue({
            graphQLErrors: [{message: "Enter the credentials again for the new address"}],
        })
        const {onClose} = renderEditor(messenger())
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
        await waitFor(() =>
            expect(screen.getByRole("alert")).toHaveTextContent(
                "messaging.reason.CREDENTIALS_FOLLOW_ENDPOINT"
            )
        )
        expect(onClose).not.toHaveBeenCalled()
    })

    it("shows a reason harvest answered as plain text", async () => {
        mockUpsert.mockRejectedValue({
            graphQLErrors: [
                {
                    message: "not a valid json response from webhook",
                    extensions: {
                        internal: {
                            response: {body: "The provider address must start with https://"},
                        },
                    },
                },
            ],
        })
        renderEditor(messenger())
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
        await waitFor(() =>
            expect(screen.getByRole("alert")).toHaveTextContent(
                "messaging.reason.ENDPOINT_NOT_HTTPS"
            )
        )
    })
})

describe("a new account (R4-08)", () => {
    const fillWhatsApp = () => {
        for (const [field, value] of [
            ["messagingAccounts.field.name", "Staff Association WhatsApp"],
            ["messagingAccounts.field.business_account_id", "1"],
            ["messagingAccounts.field.phone_number_id", "2"],
            ["messagingAccounts.field.display_phone_number", "+34 600 000 214"],
        ]) {
            fireEvent.change(screen.getByLabelText(new RegExp(field)), {target: {value}})
        }
    }

    it("is created with its credentials in one call, with nothing after it that can fail", async () => {
        mockUpsert.mockResolvedValue({
            data: {upsert_messaging_account: {id: "account-9", callback_url: CALLBACK_URL}},
        })
        const {onCreated} = renderEditor()
        fillWhatsApp()
        fireEvent.change(screen.getByLabelText("messaging.credential.ACCESS_TOKEN"), {
            target: {value: "token-1"},
        })
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
        await waitFor(() => expect(onCreated).toHaveBeenCalled())
        expect(mockUpsert.mock.calls[0][0].variables).toMatchObject({
            id: null,
            credentials: {ACCESS_TOKEN: "token-1"},
        })
        expect(mockReplace).not.toHaveBeenCalled()
    })

    it("is updated, not inserted again, when saved a second time", async () => {
        mockUpsert.mockResolvedValue({
            data: {upsert_messaging_account: {id: "account-9", callback_url: CALLBACK_URL}},
        })
        const {onCreated} = renderEditor()
        fillWhatsApp()
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
        await waitFor(() => expect(onCreated).toHaveBeenCalledTimes(1))
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
        await waitFor(() => expect(mockUpsert).toHaveBeenCalledTimes(2))
        expect(mockUpsert.mock.calls[1][0].variables.id).toBe("account-9")
    })
})

describe("Viber report credentials (R4-09)", () => {
    it("asks before replacing report credentials that are set", async () => {
        mockReplace.mockResolvedValue({
            data: {
                replace_messaging_account_credentials: {
                    replaced: ["REPORT_USERNAME", "REPORT_PASSWORD"],
                    verify_token: null,
                    report_username: "reports-2",
                    report_password: "generated-secret-2",
                },
            },
        })
        renderEditor(
            viber({
                credentials: {
                    API_KEY: {replaced_at: "2026-09-30T10:00:00Z"},
                    REPORT_USERNAME: {replaced_at: "2026-10-01T10:00:00Z"},
                    REPORT_PASSWORD: {replaced_at: "2026-10-01T10:00:00Z"},
                },
            })
        )
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.reports.generate"}))
        expect(screen.getByText("messagingAccounts.reports.confirmBody")).toBeInTheDocument()
        expect(mockReplace).not.toHaveBeenCalled()
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.reports.confirm"}))
        await waitFor(() =>
            expect(screen.getByLabelText("messaging.credential.REPORT_PASSWORD")).toHaveValue(
                "generated-secret-2"
            )
        )
        expect(mockReplace).toHaveBeenCalledTimes(1)
    })

    it("keeps the current report credentials when the replacement is cancelled", () => {
        renderEditor(
            viber({
                credentials: {
                    API_KEY: {replaced_at: "2026-09-30T10:00:00Z"},
                    REPORT_PASSWORD: {replaced_at: "2026-10-01T10:00:00Z"},
                },
            })
        )
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.reports.generate"}))
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.cancel"}))
        expect(mockReplace).not.toHaveBeenCalled()
    })
})

it("says the callback URL could not be loaded when its query fails (R4-18)", () => {
    mockCallback.mockReturnValue({data: undefined, loading: false, error: new Error("down")})
    renderEditor(messenger())
    expect(screen.getByLabelText("messagingAccounts.webhook.url")).toHaveValue(
        "messagingAccounts.webhook.loadError"
    )
})

it("says what empty limits mean, and edits the codes sent at once", async () => {
    mockUpsert.mockResolvedValue({data: {upsert_messaging_account: {id: "account-1"}}})
    renderEditor(messenger())
    expect(screen.getByText("messagingAccounts.limits.messagesPerSecondHelp")).toBeInTheDocument()
    expect(screen.getByText("messagingAccounts.limits.otpReservedHelp")).toBeInTheDocument()
    expect(screen.getByText("messagingAccounts.limits.otpConcurrencyHelp")).toBeInTheDocument()
    fireEvent.change(screen.getByLabelText("messagingAccounts.limits.otpConcurrency"), {
        target: {value: "0"},
    })
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
    expect(screen.getByText("messagingAccounts.error.AT_LEAST_ONE")).toBeInTheDocument()
    expect(mockUpsert).not.toHaveBeenCalled()
    fireEvent.change(screen.getByLabelText("messagingAccounts.limits.otpConcurrency"), {
        target: {value: "4"},
    })
    fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.editor.save"}))
    await waitFor(() =>
        expect(mockUpsert).toHaveBeenCalledWith({
            variables: expect.objectContaining({
                limits: expect.objectContaining({otp_concurrency: 4}),
            }),
        })
    )
})
