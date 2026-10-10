// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {FIXED_TIME, storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EReadinessPolicy,
} from "@/types/messaging"
import {InstantMessageChannel, InstantMessageTemplateSection} from "./InstantMessageTemplateSection"

interface Scenario {
    channel: InstantMessageChannel
    /** Whether the tenant has an account for the channel. */
    withAccount: boolean
    /** Whether an administrator confirmed the account's readiness. */
    confirmed: boolean
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

const whatsappAccount = {
    id: storyId(8, 1),
    tenant_id: TENANT_ID,
    channel: EMessageChannel.WHATSAPP,
    provider: EMessagingProvider.WHATSAPP_CLOUD_API,
    name: "Election office WhatsApp",
    sender: {
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        business_account_id: "1029384756",
        phone_number_id: "5647382910",
        display_phone_number: "+63 917 555 0100",
        display_name: "Election office",
        api_version: "v23.0",
    },
    credentials: {},
    limits: {allowed_calling_codes: []},
    provider_approval: "CONFIRMED",
    status: {
        connected: true,
        production_access: true,
        approved_templates: {[EMessagePurpose.NOTICE]: ["en"]},
        checked_at: FIXED_TIME,
    },
    webhook_key: "story-key",
    is_default: true,
    created_at: FIXED_TIME,
    updated_at: FIXED_TIME,
}

const record = {
    type: "CREDENTIALS",
    template: {
        whatsapp: {message: "Hello {{1}}, voting opens tomorrow.", parameters: ["user.first_name"]},
        messenger: {message: "Voting opens tomorrow.", parameters: []},
    },
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Template/InstantMessageTemplateSection",
    component: InstantMessageTemplateSection,
    args: {channel: EMessageChannel.WHATSAPP, withAccount: true, confirmed: false, onSubmit: fn()},
    argTypes: {
        channel: {
            control: "inline-radio",
            options: [EMessageChannel.WHATSAPP, EMessageChannel.VIBER, EMessageChannel.MESSENGER],
        },
    },
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary({
            GetMessagingAccounts: () => ({
                data: {
                    sequent_backend_messaging_account: args.withAccount
                        ? [
                              {
                                  ...whatsappAccount,
                                  readiness: args.confirmed
                                      ? EReadinessPolicy.ADMIN_CONFIRMED
                                      : EReadinessPolicy.PROVIDER_CHECK,
                              },
                          ]
                        : [],
                },
            }),
        })
        data = resourceBoundary({
            sequent_backend_tenant: [
                {
                    id: TENANT_ID,
                    settings: {
                        language_conf: {
                            enabled_language_codes: ["en", "tl"],
                            default_language_code: "en",
                        },
                    },
                },
            ],
        })
        await graphql.ready
    },
    render: ({channel, onSubmit}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <SimpleForm
                record={record}
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton alwaysEnable />
                    </Toolbar>
                }
            >
                <InstantMessageTemplateSection channel={channel} />
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const WhatsAppApprovals: Story = {
    parameters: {widgets: ["ParametersInput", "ApprovalStatus", "ApprovalTable"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("textbox", {name: /Approved wording/})).toHaveValue(
            record.template.whatsapp.message
        )
        const table = await canvas.findByRole("table", {name: "Approved templates"})
        const rows = within(table).getAllByRole("row").slice(1)
        expect(rows.map((row) => row.textContent)).toEqual(["enApproved", "tlNot approved"])
    },
}

export const EditParameters: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Add parameter"}))
        await userEvent.type(canvas.getByRole("textbox", {name: "Parameter 2"}), "vote_url")
        await userEvent.click(canvas.getByRole("button", {name: "Remove parameter 1"}))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalled())
        expect(args.onSubmit.mock.calls[0][0]).toMatchObject({
            template: {whatsapp: {parameters: ["vote_url"]}},
        })
    },
}

export const ProviderTemplate: Story = {
    parameters: {widgets: ["ProviderTemplateInputs", "ParametersInput"]},
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(/the election event's template bound to this template's alias is used/)
        ).toBeVisible()
        await expect(canvas.getByText(/write @name=value/)).toBeVisible()
        await userEvent.type(
            canvas.getByRole("textbox", {name: "Provider template name or ID"}),
            "vote_reminder"
        )
        await userEvent.type(canvas.getByRole("textbox", {name: "Provider language code"}), "en_US")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalled())
        expect(args.onSubmit.mock.calls[0][0]).toMatchObject({
            template: {
                whatsapp: {
                    message: record.template.whatsapp.message,
                    provider_template: "vote_reminder",
                    provider_language: "en_US",
                },
            },
        })
    },
}

export const ConfirmedByAnAdministrator: Story = {
    args: {confirmed: true},
    parameters: {widgets: ["ApprovalStatus"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(/An administrator confirmed with the provider/)
        ).toBeVisible()
        expect(canvas.queryByRole("table", {name: "Approved templates"})).toBeNull()
    },
}

export const ViberWithoutAccount: Story = {
    args: {channel: EMessageChannel.VIBER, withAccount: false},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(/There is no Viber account yet/)
        ).toBeVisible()
    },
}

export const MessengerWindow: Story = {
    args: {channel: EMessageChannel.MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("textbox", {name: "Message within 24 hours"})).toHaveValue(
            record.template.messenger.message
        )
        expect(canvas.getByText(/is not permission to send/)).toBeVisible()
        await expect(canvas.getByText(/page_utility_messaging permission/)).toBeVisible()
        await expect(
            canvas.getByText("The language code of the approved utility template, such as en_US.")
        ).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}
