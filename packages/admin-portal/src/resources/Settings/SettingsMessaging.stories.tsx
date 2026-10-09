// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {EMessageAttemptState, IMessagingAccount} from "@/types/messaging"
import {SettingsMessaging} from "./SettingsMessaging"
import {TENANT_RESOURCE, settingsTenant} from "./__stories__/SettingsFixture"
import {SMS_ACCOUNT_ID, messagingAccounts} from "./__stories__/MessagingFixture"

type EReads = "records" | "loading" | "error"

interface Scenario {
    reads: EReads
    empty: boolean
    roles: string[]
}

const READ_WRITE = [IPermissions.MESSAGING_ACCOUNT_READ, IPermissions.MESSAGING_ACCOUNT_WRITE]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const accountsReply = (reads: EReads, accounts: IMessagingAccount[]) => () => {
    if (reads === "loading") return new Promise<never>(() => undefined)
    if (reads === "error") throw new Error("Synthetic service unavailable")
    return {data: {sequent_backend_messaging_account: accounts}}
}

const meta = {
    title: "Admin/Settings/SettingsMessaging",
    component: SettingsMessaging,
    args: {reads: "records", empty: false, roles: READ_WRITE},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary({[TENANT_RESOURCE]: [settingsTenant()]})
        graphql = graphqlBoundary({
            GetMessagingAccounts: accountsReply(args.reads, args.empty ? [] : messagingAccounts()),
            CheckMessagingAccount: () => ({
                data: {check_messaging_account: {status: {connected: true}}},
            }),
            TestMessagingAccount: () => ({
                data: {
                    test_messaging_account: {
                        message_id: "message-1",
                        state: EMessageAttemptState.ACCEPTED,
                        reason: null,
                    },
                },
            }),
            DeleteMessagingAccount: () => ({
                data: {delete_messaging_account: {id: SMS_ACCOUNT_ID}},
            }),
        })
        await graphql.ready
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={args.roles}>
            <SettingsMessaging />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const accountRow = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

export const Accounts: Story = {
    parameters: {widgets: ["ReadinessCell"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(/Accounts that send voters their codes and notices/)
        ).toBeVisible()
        const whatsapp = await accountRow(canvasElement, "Council WhatsApp")
        await expect(within(whatsapp).getByText("Connected")).toBeVisible()
        await expect(within(whatsapp).getAllByText("Needs provider approval")).toHaveLength(2)
        await expect(within(whatsapp).getByText("Needs approved template")).toBeVisible()
        const sms = await accountRow(canvasElement, "Council SMS")
        await expect(within(sms).getByText("The account is in the SMS sandbox.")).toBeVisible()
        await expect(within(sms).getAllByText("Needs production access").length).toBe(2)
        const partner = await accountRow(canvasElement, "Partner gateway")
        await expect(within(partner).getByText("Custom HTTP API")).toBeVisible()
        await expect(within(partner).getByText("Confirmed by an administrator")).toBeVisible()
        await expect(within(partner).getByText("Ready for OTP")).toBeVisible()
        await expect(within(partner).getByText("Check not used")).toBeVisible()
        expect(within(partner).queryByText("Not connected")).toBeNull()
        const messenger = await accountRow(canvasElement, "Council Page")
        await expect(within(messenger).getByText("Not checked yet")).toBeVisible()
        expect(graphql.calls[0]).toMatchObject({
            name: "GetMessagingAccounts",
            variables: {tenantId: TENANT_ID},
            headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_READ},
        })
    },
}

export const ReadOnly: Story = {
    args: {roles: [IPermissions.MESSAGING_ACCOUNT_READ]},
    play: async ({canvasElement}) => {
        const row = await accountRow(canvasElement, "Council Viber")
        expect(within(row).queryByRole("button", {name: /Check the connection/})).toBeNull()
        await expect(within(row).getByRole("button", {name: "View Council Viber"})).toBeVisible()
        expect(within(canvasElement).queryByRole("button", {name: "Add account"})).toBeNull()
    },
}

export const CheckConnection: Story = {
    play: async ({canvasElement}) => {
        const row = await accountRow(canvasElement, "Council SMS")
        await userEvent.click(
            within(row).getByRole("button", {name: "Check the connection of Council SMS"})
        )
        await waitFor(() =>
            expect(graphql.calls.map(({name}) => name)).toEqual([
                "GetMessagingAccounts",
                "CheckMessagingAccount",
                "GetMessagingAccounts",
            ])
        )
        expect(graphql.calls[1]).toMatchObject({
            variables: {id: SMS_ACCOUNT_ID},
            headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_WRITE},
        })
    },
}

export const TestMessage: Story = {
    play: async ({canvasElement}) => {
        const row = await accountRow(canvasElement, "Council SMS")
        await userEvent.click(
            within(row).getByRole("button", {name: "Send a test message from Council SMS"})
        )
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.type(
            dialog.getByRole("textbox", {name: "Phone number (E.164)"}),
            "+34600000001"
        )
        await userEvent.click(dialog.getByRole("button", {name: "Send test message"}))
        await expect(await dialog.findByText("Accepted")).toBeVisible()
        await expect(dialog.getByText(/This does not mean the voter received it/)).toBeVisible()
    },
}

export const DeleteAccount: Story = {
    play: async ({canvasElement}) => {
        const row = await accountRow(canvasElement, "Council SMS")
        await userEvent.click(within(row).getByRole("button", {name: "Delete Council SMS"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(dialog.getByRole("button", {name: "Delete"}))
        await waitFor(() =>
            expect(graphql.calls.map(({name}) => name)).toContain("DeleteMessagingAccount")
        )
    },
}

export const AddAccount: Story = {
    play: async ({canvasElement}) => {
        await accountRow(canvasElement, "Council SMS")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Add account"}))
        const drawer = within(await within(document.body).findByRole("presentation"))
        await expect(await drawer.findByRole("textbox", {name: /From address/})).toBeVisible()
    },
}

export const Empty: Story = {
    args: {empty: true},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("No sending accounts yet.")
        ).toBeVisible()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("progressbar", {name: "Loading accounts"})
        ).toBeVisible()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("The sending accounts could not be loaded.")
        ).toBeVisible()
    },
}
