// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {EMessagingProvider, EProviderApproval, IMessagingAccount} from "@/types/messaging"
import {MessagingAccountEditor} from "./MessagingAccountEditor"
import {
    MESSENGER_ACCOUNT_ID,
    VIBER_ACCOUNT_ID,
    WHATSAPP_ACCOUNT_ID,
    callbackUrl,
    callbackUrlHandler,
    messengerAccount,
    partnerAccount,
    viberAccount,
    whatsappAccount,
} from "./__stories__/MessagingFixture"

enum EStoryAccount {
    NEW = "NEW",
    WHATSAPP = "WHATSAPP",
    VIBER = "VIBER",
    MESSENGER = "MESSENGER",
    /** An account of the removed generic HTTP provider. */
    UNSUPPORTED = "UNSUPPORTED",
    /** WhatsApp, with everything the provider's check reported. */
    WHATSAPP_CHECKED = "WHATSAPP_CHECKED",
    /** WhatsApp after its number changed, before the check ran again. */
    WHATSAPP_UNCHECKED = "WHATSAPP_UNCHECKED",
}

/** What the provider's check reported for the WhatsApp account. */
const whatsappChecked = (): IMessagingAccount => {
    const account = whatsappAccount()
    return {
        ...account,
        status: {
            connected: true,
            production_access: true,
            approved_templates: {OTP: ["en"], NOTICE: ["en"]},
            checked_at: "2026-10-09T08:00:00Z",
            reason: null,
            details: {
                token: {
                    valid: true,
                    expires_at: "2026-12-01T00:00:00Z",
                    missing_scopes: [],
                },
                whatsapp_number: {
                    status: "CONNECTED",
                    name_status: "APPROVED",
                    quality_rating: "GREEN",
                    messaging_limit: "TIER_1K",
                },
                templates: [
                    {
                        name: "voting_code",
                        language: "en",
                        category: "AUTHENTICATION",
                        status: "APPROVED",
                    },
                    {
                        name: "election_notice",
                        language: "en",
                        category: "UTILITY",
                        status: "APPROVED",
                    },
                    {
                        name: "election_notice",
                        language: "es",
                        category: "UTILITY",
                        status: "PENDING",
                    },
                ],
                provider_approval: EProviderApproval.CONFIRMED,
                number_changes: [
                    {
                        masked_old_number: "+34*******101",
                        masked_new_number: "+34*******202",
                        voter_ids: ["voter-0001"],
                        reported_at: "2026-10-08T17:30:00Z",
                    },
                ],
            },
        },
    }
}

/** Its last check was of the number it had before. */
const whatsappUnchecked = (): IMessagingAccount => {
    const account = whatsappChecked()
    return {
        ...account,
        status: account.status && {
            ...account.status,
            details: {
                ...account.status.details,
                identity: {
                    provider: EMessagingProvider.WHATSAPP_CLOUD_API,
                    ids: {business_account_id: "100000000000001", phone_number_id: "earlier"},
                },
            },
        },
    }
}

interface Scenario {
    account: EStoryAccount
    canWrite: boolean
    mutationsFail: boolean
}

const ACCOUNTS: Record<EStoryAccount, () => IMessagingAccount | undefined> = {
    [EStoryAccount.NEW]: () => undefined,
    [EStoryAccount.WHATSAPP]: whatsappAccount,
    [EStoryAccount.VIBER]: viberAccount,
    [EStoryAccount.MESSENGER]: messengerAccount,
    [EStoryAccount.UNSUPPORTED]: partnerAccount,
    [EStoryAccount.WHATSAPP_CHECKED]: whatsappChecked,
    [EStoryAccount.WHATSAPP_UNCHECKED]: whatsappUnchecked,
}

let graphql: ReturnType<typeof graphqlBoundary>
const onChanged = fn()
const onClose = fn()
const onCreated = fn()

const reply = (fails: boolean, data: Record<string, unknown>) => () => {
    if (fails) throw new Error("Synthetic messaging service unavailable")
    return {data}
}

const meta = {
    title: "Admin/Settings/MessagingAccountEditor",
    component: MessagingAccountEditor,
    args: {account: EStoryAccount.WHATSAPP, canWrite: true, mutationsFail: false},
    argTypes: {account: {control: "inline-radio", options: Object.values(EStoryAccount)}},
    parameters: {widgets: ["CredentialInput", "CallbackSection"]},
    beforeEach: async ({args}) => {
        onChanged.mockClear()
        onClose.mockClear()
        onCreated.mockClear()
        graphql = graphqlBoundary({
            UpsertMessagingAccount: reply(args.mutationsFail, {
                upsert_messaging_account: {
                    id: MESSENGER_ACCOUNT_ID,
                    callback_url: callbackUrl("meta", MESSENGER_ACCOUNT_ID),
                    callback_reason: null,
                },
            }),
            ReplaceMessagingAccountCredentials: reply(args.mutationsFail, {
                replace_messaging_account_credentials: {
                    replaced: ["VERIFY_TOKEN", "REPORT_USERNAME", "REPORT_PASSWORD"],
                    verify_token: "synthetic-verify-token",
                    report_username: "synthetic-report-user",
                    report_password: "synthetic-report-password",
                },
            }),
            MessagingAccountCallbackUrl: callbackUrlHandler,
            MessagingAccountChangeImpact: reply(args.mutationsFail, {
                messaging_account_change_impact: {affected_voters: 37, events: 2},
            }),
        })
        await graphql.ready
    },
    render: (args) => (
        <AdminStoryProvider
            boundary={graphql}
            roles={args.canWrite ? [IPermissions.MESSAGING_ACCOUNT_WRITE] : []}
        >
            <MessagingAccountEditor
                key={args.account}
                account={ACCOUNTS[args.account]()}
                canWrite={args.canWrite}
                onChanged={onChanged}
                onCreated={onCreated}
                onClose={onClose}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const WhatsApp: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("textbox", {name: /WhatsApp Business Account ID/})
        ).toHaveValue("100000000000001")
        await expect(canvas.getAllByText(/Set · replaced/).length).toBeGreaterThan(0)
        expect(canvas.queryByLabelText("Access token")).toBeNull()
        await expect(
            canvas.getByText(/Choose Provider approval confirmed once Meta has approved it/)
        ).toBeVisible()
        await expect(canvas.getByRole("combobox", {name: "Provider approval"})).not.toHaveAttribute(
            "aria-disabled"
        )
        await expect(canvas.getByRole("textbox", {name: "Graph API base URL"})).toHaveValue("")
        await userEvent.clear(canvas.getByRole("textbox", {name: /Phone number ID/}))
        await userEvent.type(canvas.getByRole("textbox", {name: /Phone number ID/}), "300")
        await expect(canvas.getByText(/Messages will come from another number/)).toBeVisible()
    },
}

export const GenerateVerifyToken: Story = {
    parameters: {widgets: ["CallbackSection", "VerifyTokenDialog"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByDisplayValue(callbackUrl("meta", WHATSAPP_ACCOUNT_ID))
        ).toBeVisible()
        await expect(
            canvas.getByText(/Delivery reports arrive here and are checked with the app secret/)
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Generate verify token"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(dialog.getByRole("textbox", {name: "Verify token"})).toHaveValue(
            "synthetic-verify-token"
        )
        expect(graphql.calls[0]).toMatchObject({
            name: "MessagingAccountCallbackUrl",
            variables: {accountId: WHATSAPP_ACCOUNT_ID},
            headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_WRITE},
        })
        expect(graphql.calls[1]).toMatchObject({
            name: "ReplaceMessagingAccountCredentials",
            variables: {credentials: {}, generateVerifyToken: true},
            headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_WRITE},
        })
        await userEvent.click(dialog.getByRole("button", {name: "Done"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const ViberTemplates: Story = {
    args: {account: EStoryAccount.VIBER},
    parameters: {widgets: ["ViberTemplatesEditor", "CallbackSection", "CredentialInput"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getAllByRole("textbox", {name: "Partner template ID"})).toHaveLength(3)
        await expect(
            await canvas.findByDisplayValue(callbackUrl("viber", VIBER_ACCOUNT_ID))
        ).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Generate verify token"})).toBeNull()
        expect(canvas.queryByText(/app secret/i)).toBeNull()
    },
}

export const ViberReportCredentials: Story = {
    args: {account: EStoryAccount.VIBER},
    parameters: {
        widgets: [
            "CallbackSection",
            "ReportCredentialsDialog",
            "CopyField",
            "ViberSubscriptionSteps",
        ],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(/create a subscription for Viber delivery reports/)
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Generate report credentials"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(dialog.getByRole("textbox", {name: "Report user name"})).toHaveValue(
            "synthetic-report-user"
        )
        await expect(dialog.getByRole("textbox", {name: "Report password"})).toHaveValue(
            "synthetic-report-password"
        )
        await expect(dialog.getByRole("textbox", {name: "Callback URL"})).toHaveValue(
            callbackUrl("viber", VIBER_ACCOUNT_ID)
        )
        expect(graphql.calls[1]).toMatchObject({
            name: "ReplaceMessagingAccountCredentials",
            variables: {generateReportCredentials: true, generateVerifyToken: false},
        })
        await userEvent.click(dialog.getByRole("button", {name: "Done"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const ChangeMessengerPage: Story = {
    args: {account: EStoryAccount.MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const page = canvas.getByRole("textbox", {name: /Facebook Page ID/})
        await userEvent.clear(page)
        await userEvent.type(page, "999")
        await expect(
            canvas.getByText(
                /codes and notices to the voters connected to Council Elections aren't sent until they connect again/
            )
        ).toBeVisible()
        await expect(await canvas.findByText(/who must connect again: 37/)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(onClose).toHaveBeenCalled())
        expect(graphql.calls.find(({name}) => name === "UpsertMessagingAccount")).toMatchObject({
            variables: {sender: {provider: "MESSENGER_SEND_API", page_id: "999"}},
        })
        expect(onChanged).toHaveBeenCalled()
    },
}

export const ProviderStatus: Story = {
    args: {account: EStoryAccount.WHATSAPP_CHECKED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Provider status")).toBeVisible()
        await expect(canvas.getByText("High")).toBeVisible()
        await expect(canvas.getByText("TIER_1K")).toBeVisible()
        await expect(canvas.getByText(/Reported by the provider's check/)).toBeVisible()
        // The check reports the approval: there is nothing to choose.
        expect(canvas.queryByRole("combobox", {name: "Provider approval"})).toBeNull()
        await expect(canvas.getByText("voting_code")).toBeVisible()
        await expect(canvas.getByText("In review")).toBeVisible()
        await expect(canvas.getByText("Voters who changed their WhatsApp number")).toBeVisible()
        await expect(canvas.getByText(/Voters: voter-0001/)).toBeVisible()
    },
}

export const IdentityChangedNotChecked: Story = {
    args: {account: EStoryAccount.WHATSAPP_UNCHECKED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(/Until the account is checked again, codes and notices/)
        ).toBeVisible()
        expect(canvas.queryByText("Quality rating")).toBeNull()
    },
}

export const NewAccount: Story = {
    args: {account: EStoryAccount.NEW},
    parameters: {widgets: ["CredentialInput", "CallbackSection"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("textbox", {name: /Phone number ID/})).toBeVisible()
        await expect(canvas.getByRole("textbox", {name: "Callback URL"})).toHaveValue(
            "Shown after saving"
        )
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await expect(canvas.getAllByText("Required").length).toBeGreaterThan(0)
        expect(graphql.calls).toEqual([])
    },
}

const choose = async (canvasElement: HTMLElement, select: string, option: string) => {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: select}))
    await userEvent.click(await within(document.body).findByRole("option", {name: option}))
}

export const ChannelsAndTheirAdapters: Story = {
    args: {account: EStoryAccount.NEW},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("combobox", {name: "Channel"}))
        const channels = within(document.body)
            .getAllByRole("option")
            .map((option) => option.textContent)
        await expect(channels).toEqual(["WhatsApp", "Viber", "Facebook Messenger", "SMS", "Email"])
        await userEvent.click(within(document.body).getByRole("option", {name: "SMS"}))
        await userEvent.click(canvas.getByRole("combobox", {name: "Provider"}))
        const providers = within(document.body)
            .getAllByRole("option")
            .map((option) => option.textContent)
        await expect(providers).toEqual(["Amazon SNS"])
        await userEvent.keyboard("{Escape}")
    },
}

export const UnsupportedProvider: Story = {
    args: {account: EStoryAccount.UNSUPPORTED},
    parameters: {
        widgets: [],
        expectedFailure: {
            reason: "Disabled fields draw their helper text in the theme's disabled grey.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("alert")).toHaveTextContent(
            /Unsupported provider: replace this account/
        )
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
        expect(canvas.queryByRole("textbox", {name: "Callback URL"})).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

export const NewAccountShowsItsCallback: Story = {
    args: {account: EStoryAccount.NEW},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await choose(canvasElement, "Channel", "Facebook Messenger")
        await userEvent.type(canvas.getByRole("textbox", {name: /Account name/}), "Council Page")
        await userEvent.type(canvas.getByRole("textbox", {name: /Facebook Page ID/}), "42")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(onCreated).toHaveBeenCalledWith(MESSENGER_ACCOUNT_ID, {
                callback_url: callbackUrl("meta", MESSENGER_ACCOUNT_ID),
                reason: null,
            })
        )
        expect(onClose).not.toHaveBeenCalled()
        expect("readiness" in (graphql.calls[0].variables ?? {})).toBe(false)
    },
}

export const SaveFails: Story = {
    args: {account: EStoryAccount.MESSENGER, mutationsFail: true},
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        const notice = await within(document.body).findByText("The account could not be saved.")
        await waitFor(() => expect(notice).toBeVisible())
        expect(onClose).not.toHaveBeenCalled()
    },
}

export const ReadOnly: Story = {
    args: {canWrite: false},
    parameters: {
        expectedFailure: {
            reason: "Disabled fields draw their helper text in the theme's disabled grey.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("textbox", {name: /Phone number ID/})).toBeDisabled()
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Replace"})).toBeNull()
    },
}

const checkViberTemplateLayout = async (canvasElement: HTMLElement) => {
    const section = within(canvasElement).getByRole("heading", {name: "Approved templates"})
        .parentElement as HTMLElement
    const originalWidth = section.style.width
    try {
        for (const width of [320, 512]) {
            section.style.width = `${width}px`
            await waitFor(() => {
                const bounds = section.getBoundingClientRect()
                for (const field of Array.from(section.querySelectorAll(".MuiFormControl-root"))) {
                    const rect = field.getBoundingClientRect()
                    expect(rect.width).toBeGreaterThanOrEqual(100)
                    expect(rect.left).toBeGreaterThanOrEqual(bounds.left)
                    expect(rect.right).toBeLessThanOrEqual(bounds.right + 1)
                }
                const template = within(section)
                    .getAllByRole("textbox", {
                        name: "Partner template ID",
                    })[0]
                    .closest(".MuiFormControl-root") as HTMLElement
                expect(template.getBoundingClientRect().width).toBeGreaterThanOrEqual(180)
            })
        }
    } finally {
        section.style.width = originalWidth
    }
}

export const ResponsiveViberEdit: Story = {
    args: {account: EStoryAccount.VIBER},
    play: async ({canvasElement}) => {
        await checkViberTemplateLayout(canvasElement)
    },
}

export const ResponsiveViberCreate: Story = {
    args: {account: EStoryAccount.NEW},
    play: async ({canvasElement}) => {
        await choose(canvasElement, "Channel", "Viber")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Add template"}))
        await checkViberTemplateLayout(canvasElement)
        const canvas = within(canvasElement)
        await userEvent.type(canvas.getByRole("textbox", {name: "Language"}), "en")
        await userEvent.type(canvas.getByRole("textbox", {name: "Partner template ID"}), "otp_en")
        await expect(canvas.getByRole("textbox", {name: "Partner template ID"})).toHaveValue(
            "otp_en"
        )
        await userEvent.click(canvas.getByRole("button", {name: "Remove template"}))
        expect(canvas.queryByRole("textbox", {name: "Partner template ID"})).toBeNull()
    },
}
