// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {Typography} from "@mui/material"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {MessagingTabs} from "./MessagingTabs"
import {MESSENGER_LINK, fakeMessageHandler, sentMessages} from "./__stories__/SentMessagesFixture"

interface Scenario {
    /** Whether harvest mounts the fake provider routes (the action answers). */
    routes: "mounted" | "absent"
    roles: string[]
    refresh: "manual" | "automatic"
    messages: "populated" | "arriving"
}

const ACCOUNTS = "The sending accounts"
const WRITE_ROLE = {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_WRITE}

let graphql: ReturnType<typeof graphqlBoundary>
let messagesArrived = false

const callsOf = (name: string) => graphql.calls.filter((call) => call.name === name)

/** The story router's location (path and query), from the preview's status element. */
const location = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("status", {name: "Current location"}).textContent ?? ""

const meta = {
    title: "Admin/Settings/Sent messages/MessagingTabs",
    component: MessagingTabs,
    args: {
        routes: "mounted",
        refresh: "manual",
        messages: "populated",
        roles: [IPermissions.MESSAGING_ACCOUNT_READ, IPermissions.MESSAGING_ACCOUNT_WRITE],
    },
    parameters: {router: {initialEntries: ["/settings?tabIndex=2"]}},
    argTypes: {routes: {control: "inline-radio", options: ["mounted", "absent"]}},
    beforeEach: ({args}) => {
        messagesArrived = false
        graphql = graphqlBoundary({
            MessagingFakeProviderMessages: () => {
                if (args.routes === "absent") {
                    throw new Error("Synthetic http exception when calling webhook")
                }
                const messages =
                    args.messages === "arriving" && !messagesArrived ? [] : sentMessages()
                return {data: {messaging_fake_provider_messages: {messages}}}
            },
            MessagingFakeProviderMessage: fakeMessageHandler,
            MessagingFakeProviderGetStarted: () => ({
                data: {messaging_fake_provider_get_started: {status: 200, reason: null}},
            }),
        })
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql} roles={args.roles}>
            <MessagingTabs
                accounts={<Typography>{ACCOUNTS}</Typography>}
                refreshMs={args.refresh === "automatic" ? undefined : 0}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const SentMessagesTab: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const tab = await canvas.findByRole("tab", {name: "Sent messages"})
        await expect(canvas.getByRole("tab", {name: "Accounts"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect(canvas.getByText(ACCOUNTS)).toBeVisible()
        await userEvent.click(tab)
        await expect(await canvas.findByRole("table", {name: "Sent messages"})).toBeVisible()
        await waitFor(() =>
            expect(location(canvasElement)).toBe("/settings?tabIndex=2&channel=all")
        )
        await userEvent.click(canvas.getByRole("button", {name: "Email"}))
        await expect(await canvas.findByRole("list", {name: "Inbox"})).toBeVisible()
        await waitFor(() =>
            expect(location(canvasElement)).toBe("/settings?tabIndex=2&channel=email")
        )
        await userEvent.click(canvas.getByRole("button", {name: "Refresh"}))
        await waitFor(() => expect(callsOf("MessagingFakeProviderMessages")).toHaveLength(2))
        expect(callsOf("MessagingFakeProviderMessages")[0].headers).toMatchObject(WRITE_ROLE)
        await userEvent.click(canvas.getByRole("tab", {name: "Accounts"}))
        await expect(await canvas.findByText(ACCOUNTS)).toBeVisible()
        await waitFor(() => expect(location(canvasElement)).toBe("/settings?tabIndex=2"))
    },
}

export const MessengerLink: Story = {
    parameters: {router: {initialEntries: ["/settings?channel=messenger"]}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const form = within(await canvas.findByRole("form", {name: "Get Started"}))
        await userEvent.type(form.getByRole("textbox", {name: "Messenger link"}), MESSENGER_LINK)
        await userEvent.click(form.getByRole("button", {name: "Get Started"}))
        await expect(
            await form.findByText(
                "The webhook component answered 200. The code arrives in the chat."
            )
        ).toBeVisible()
        expect(callsOf("MessagingFakeProviderGetStarted")[0]).toMatchObject({
            variables: {link: MESSENGER_LINK},
            headers: WRITE_ROLE,
        })
        // The list is read again, for the code that follows.
        await waitFor(() => expect(callsOf("MessagingFakeProviderMessages")).toHaveLength(2))
    },
}

export const RoutesAbsent: Story = {
    args: {routes: "absent"},
    parameters: {router: {initialEntries: ["/settings?channel=whatsapp"]}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() => expect(callsOf("MessagingFakeProviderMessages")).toHaveLength(1))
        await expect(await canvas.findByText(ACCOUNTS)).toBeVisible()
        expect(canvas.queryByRole("tablist")).toBeNull()
    },
}

export const WithoutWriteRole: Story = {
    args: {roles: [IPermissions.MESSAGING_ACCOUNT_READ]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(ACCOUNTS)).toBeVisible()
        expect(canvas.queryByRole("tablist")).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

export const AutomaticRefresh: Story = {
    args: {messages: "arriving", refresh: "automatic"},
    parameters: {router: {initialEntries: ["/settings?channel=all"]}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No messages yet.")).toBeVisible()
        const refresh = canvas.getByRole("button", {name: "Refresh"})
        await expect(refresh).toBeEnabled()
        messagesArrived = true
        await waitFor(
            () => {
                const table = canvas.getByRole("table", {name: "Sent messages"})
                expect(within(table).getAllByRole("row")).toHaveLength(sentMessages().length + 1)
            },
            {timeout: 10000}
        )
        const reads = callsOf("MessagingFakeProviderMessages").length
        await userEvent.click(refresh)
        await waitFor(() =>
            expect(callsOf("MessagingFakeProviderMessages").length).toBeGreaterThan(reads)
        )
        await waitFor(() => expect(refresh).toBeEnabled())
        await userEvent.click(canvas.getByRole("tab", {name: "Accounts"}))
        await expect(canvas.getByText(ACCOUNTS)).toBeVisible()
    },
}
