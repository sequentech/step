// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {EMessageChannel} from "@/types/messaging"
import {EmailInbox} from "./EmailInbox"
import {EMAIL_VOTER, channelMessages, fakeMessageHandler} from "./__stories__/SentMessagesFixture"

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Settings/Sent messages/EmailInbox",
    component: EmailInbox,
    args: {messages: channelMessages(EMessageChannel.EMAIL)},
    beforeEach: () => {
        graphql = graphqlBoundary({MessagingFakeProviderMessage: fakeMessageHandler})
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql}>
            <EmailInbox {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof EmailInbox>
export default meta
type Story = StoryObj<typeof meta>

export const Inbox: Story = {
    parameters: {widgets: ["EmailReadingPane"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const inbox = within(canvas.getByRole("list", {name: "Inbox"}))
        // Newest first, and the newest is open.
        const items = inbox.getAllByRole("button")
        await expect(items[0]).toHaveTextContent("Your verification code")
        const pane = within(canvas.getByRole("article", {name: "Selected email"}))
        await expect(pane.getByRole("heading", {name: "Your verification code"})).toBeVisible()
        await expect(pane.getByText("From: no-reply@council.example.org")).toBeVisible()
        await expect(pane.getByText(`To: ${EMAIL_VOTER}`)).toBeVisible()
        await expect(pane.getByText("No delivery reports on this channel")).toBeVisible()
        // The HTML is in a sandboxed frame: no scripts, forms or same-origin access.
        const frame = await pane.findByTitle("Email content")
        await expect(frame).toHaveAttribute("sandbox", "")
        await expect(frame.getAttribute("srcdoc")).toContain("<b>845120</b>")
        // Nothing loads from outside: the CSP meta comes first (R6-06).
        await expect(frame.getAttribute("srcdoc")).toMatch(
            /^<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'">/
        )
        // The HTML was read for the open email only (R6-03).
        expect(
            graphql.calls
                .filter(({name}) => name === "MessagingFakeProviderMessage")
                .map(({variables}) => variables.id)
        ).toEqual(["fake-9"])
        await userEvent.click(pane.getByRole("button", {name: "Plain text"}))
        await expect(pane.getByText("Your verification code is 845120.")).toBeVisible()
        await expect(pane.queryByTitle("Email content")).toBeNull()
        // A text-only email.
        await userEvent.click(items[1])
        const other = within(canvas.getByRole("article", {name: "Selected email"}))
        await expect(other.getByText(/We received your enrollment/)).toBeVisible()
        await expect(
            other.getByText("Cut: the demo provider keeps only the first 64 KiB of each field.")
        ).toBeVisible()
        expect(other.queryByTitle("Email content")).toBeNull()
    },
}

export const NoEmails: Story = {
    args: {messages: []},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No Email messages yet.")).toBeVisible()
    },
}
