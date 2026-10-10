// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Settings > Messaging of a second organization (ui-drafts "Second org",
// msg-a-fixture-accounts.png): the same screen with its own accounts on all
// five channels, its senders and its monthly volumes.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    OTHER_CLIENT,
    SECOND_ORG_ACCOUNT_NAMES,
    SECOND_ORG_EMAIL_FROM,
    SECOND_ORG_SENDERS,
    SECOND_ORG_WHATSAPP_NUMBER,
    secondOrgAccounts,
    secondOrgCallbackUrlHandler,
    secondOrgMonthlyVolumes,
    secondOrgSentMessages,
    secondOrganizationTenant,
} from "@/__stories__/SecondOrganizationFixture"
import {TENANT_RESOURCE} from "@/resources/Settings/__stories__/SettingsFixture"
import {SettingsMessaging} from "@/resources/Settings/SettingsMessaging"
import {IPermissions} from "@/types/keycloak"
import {EMessageChannel} from "@/types/messaging"

interface Scenario {
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Screens/Admin/Second organization/Settings messaging",
    component: SettingsMessaging,
    args: {roles: [IPermissions.MESSAGING_ACCOUNT_READ, IPermissions.MESSAGING_ACCOUNT_WRITE]},
    beforeEach: async () => {
        data = resourceBoundary({[TENANT_RESOURCE]: [secondOrganizationTenant()]})
        // The messaging operations are not in the generated schema yet.
        graphql = graphqlBoundary(
            {
                GetMessagingAccounts: () => ({
                    data: {sequent_backend_messaging_account: secondOrgAccounts()},
                }),
                GetMessagingCallbackStatus: () => ({
                    data: {messaging_callback_status: {accounts: []}},
                }),
                MessagingAccountCallbackUrl: secondOrgCallbackUrlHandler,
                MessagingAccountChangeImpact: () => ({
                    data: {messaging_account_change_impact: {affected_voters: 0, events: 1}},
                }),
                GetMessagingMonthlyVolumes: ({variables}) => ({
                    data: {
                        messaging_monthly_volumes: {
                            month: variables.month,
                            rows: secondOrgMonthlyVolumes(),
                        },
                    },
                }),
                // The dev fake provider routes are mounted: the Sent messages tab shows.
                MessagingFakeProviderMessages: () => ({
                    data: {messaging_fake_provider_messages: {messages: secondOrgSentMessages()}},
                }),
                MessagingFakeProviderGetStarted: () => ({
                    data: {messaging_fake_provider_get_started: {status: 200, reason: null}},
                }),
            },
            {schema: false}
        )
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles}>
            <SettingsMessaging sentMessagesRefreshMs={0} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const accountTable = async (canvasElement: HTMLElement) =>
    within(await within(canvasElement).findByRole("table", {name: "Sending accounts"}))

const noOtherClient = (element: HTMLElement) =>
    expect(element.textContent).not.toMatch(OTHER_CLIENT)

/** Its accounts on every channel, in channel order, with its own senders and providers. */
export const Accounts: Story = {
    play: async ({canvasElement}) => {
        const table = await accountTable(canvasElement)
        await table.findByText(SECOND_ORG_ACCOUNT_NAMES.EMAIL)
        const rows = table.getAllByRole("row").slice(1)
        const cell = (row: HTMLElement, index: number) =>
            row.querySelectorAll("td")[index]?.textContent
        expect(rows.map((row) => cell(row, 1))).toEqual([
            SECOND_ORG_ACCOUNT_NAMES.WHATSAPP,
            SECOND_ORG_ACCOUNT_NAMES.VIBER,
            SECOND_ORG_ACCOUNT_NAMES.MESSENGER,
            SECOND_ORG_ACCOUNT_NAMES.SMS,
            SECOND_ORG_ACCOUNT_NAMES.EMAIL,
        ])
        expect(rows.map((row) => cell(row, 2))).toEqual([
            SECOND_ORG_SENDERS.WHATSAPP,
            SECOND_ORG_SENDERS.VIBER,
            SECOND_ORG_SENDERS.MESSENGER,
            SECOND_ORG_SENDERS.SMS,
            SECOND_ORG_SENDERS.EMAIL,
        ])
        // Email through the organization's own SMTP server.
        expect(cell(rows[4], 3)).toBe("SMTP server")
        for (const row of rows) {
            await expect(within(row).getByText("Connected")).toBeVisible()
        }
        noOtherClient(canvasElement)
    },
}

/** Its monthly volumes, under the accounts, name its own accounts. */
export const MonthlyVolumes: Story = {
    play: async ({canvasElement}) => {
        const volumes = within(
            await within(canvasElement).findByRole("table", {name: "Monthly volumes"})
        )
        await expect(await volumes.findByText(SECOND_ORG_ACCOUNT_NAMES.SMS)).toBeVisible()
        expect(
            volumes
                .getAllByRole("row")
                .slice(1, -1)
                .map((row) => row.getAttribute("data-channel"))
        ).toEqual([
            EMessageChannel.WHATSAPP,
            EMessageChannel.VIBER,
            EMessageChannel.SMS,
            EMessageChannel.EMAIL,
        ])
        await expect(volumes.getByText(SECOND_ORG_ACCOUNT_NAMES.EMAIL)).toBeVisible()
        expect(volumes.getAllByText("Spain (ES)")).toHaveLength(3)
        noOtherClient(canvasElement)
    },
}

/** The WhatsApp account's drawer: its own number and display name. */
export const WhatsAppAccountDrawer: Story = {
    play: async ({canvasElement}) => {
        const table = await accountTable(canvasElement)
        await userEvent.click(
            await table.findByRole("button", {name: `Edit ${SECOND_ORG_ACCOUNT_NAMES.WHATSAPP}`})
        )
        const drawer = await within(document.body).findByRole("presentation")
        const panel = within(drawer)
        await expect(await panel.findByText("Edit WhatsApp account")).toBeVisible()
        // The account's name, and the display name Meta approved for its number.
        await expect(panel.getByRole("textbox", {name: /^Account name/})).toHaveValue(
            SECOND_ORG_ACCOUNT_NAMES.WHATSAPP
        )
        await expect(panel.getByRole("textbox", {name: /^Display name/})).toHaveValue(
            SECOND_ORG_SENDERS.WHATSAPP
        )
        await expect(panel.getByDisplayValue(SECOND_ORG_WHATSAPP_NUMBER)).toBeVisible()
        noOtherClient(canvasElement)
        noOtherClient(drawer)
    },
}

/** The email account's drawer: SMTP, with its own From address and server. */
export const EmailAccountDrawer: Story = {
    play: async ({canvasElement}) => {
        const table = await accountTable(canvasElement)
        await userEvent.click(
            await table.findByRole("button", {name: `Edit ${SECOND_ORG_ACCOUNT_NAMES.EMAIL}`})
        )
        const drawer = await within(document.body).findByRole("presentation")
        const panel = within(drawer)
        await expect(await panel.findByText("Edit Email account")).toBeVisible()
        await expect(panel.getByDisplayValue(SECOND_ORG_EMAIL_FROM)).toBeVisible()
        await expect(panel.getByDisplayValue(SECOND_ORG_SENDERS.EMAIL)).toBeVisible()
        await expect(
            panel.getByDisplayValue("smtp.staffassoc.admin-story.invalid:587")
        ).toBeVisible()
        noOtherClient(canvasElement)
        noOtherClient(drawer)
    },
}

/** The Sent messages tab lists what the organization's accounts sent. */
export const SentMessagesTab: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("tab", {name: "Sent messages"}))
        const table = within(await canvas.findByRole("table", {name: "Sent messages"}))
        await waitFor(() => expect(table.getAllByRole("row").slice(1)).toHaveLength(3))
        await expect(table.getByText("Staff Association Council 2028: voting opens")).toBeVisible()
        await expect(table.getByText(/^Your Staff Association verification code/)).toBeVisible()
        noOtherClient(canvasElement)
    },
}
