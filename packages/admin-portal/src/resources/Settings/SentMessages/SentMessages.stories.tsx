// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {EMessageChannel} from "@/types/messaging"
import {SentMessages} from "./SentMessages"
import {
    EMAIL_VOTER,
    MESSENGER_VOTER,
    SMS_VOTER,
    VIBER_VOTER,
    WHATSAPP_CODE,
    WHATSAPP_VOTER,
    sentMessages,
    chatButton,
    fakeMessageHandler,
} from "./__stories__/SentMessagesFixture"

let graphql: ReturnType<typeof graphqlBoundary>

// Each mode from fixtures; nothing polls (the viewer's data comes as props).
const meta = {
    title: "Admin/Settings/Sent messages/SentMessages",
    component: SentMessages,
    args: {
        messages: sentMessages(),
        mode: "ALL",
        onModeChange: fn(),
        onRefresh: fn(),
        refreshing: false,
        refreshFailed: false,
        onGetStarted: fn(async () => ({status: 200, reason: null})),
    },
    beforeEach: () => {
        graphql = graphqlBoundary({MessagingFakeProviderMessage: fakeMessageHandler})
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql}>
            <SentMessages {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof SentMessages>
export default meta
type Story = StoryObj<typeof meta>

export const All: Story = {
    parameters: {widgets: ["AllMessagesTable"]},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const table = canvas.getByRole("table", {name: "Sent messages"})
        const rows = within(table).getAllByRole("row").slice(1)
        await expect(rows).toHaveLength(sentMessages().length)
        // Newest first.
        await expect(within(rows[0]).getByText("Your verification code")).toBeVisible()
        await expect(within(rows[0]).getByText("No delivery reports on this channel")).toBeVisible()
        await expect(
            canvas.getAllByRole("img", {
                name: "Report accepted by the webhook component (HTTP 200)",
            })
        ).toHaveLength(3)
        await expect(canvas.getByText("Report refused: HTTP 401")).toBeVisible()
        await expect(canvas.getByText("Report pending")).toBeVisible()
        await expect(
            canvas.getByText("The report got no answer: The webhook component didn't answer")
        ).toBeVisible()
        await expect(
            canvas.getByText("Report not sent: Several accounts use this sender")
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Viber"}))
        await expect(args.onModeChange).toHaveBeenCalledWith(EMessageChannel.VIBER)
        await userEvent.click(canvas.getByRole("button", {name: "Refresh"}))
        await expect(args.onRefresh).toHaveBeenCalled()
    },
}

const FULL_TEXT =
    "Voting is open.\n" +
    "Please review every candidate before casting your ballot. ".repeat(20) +
    "End of notification."
const expandedMessages = sentMessages().map((message) =>
    message.channel === EMessageChannel.SMS
        ? {...message, content: {...message.content, text: FULL_TEXT}}
        : message
)

export const ExpandMessage: Story = {
    args: {messages: expandedMessages},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const row = canvas.getAllByRole("row").find((row) => row.textContent?.includes(SMS_VOTER))
        if (!row) throw new Error("SMS row is missing")
        const expand = within(row).getByRole("button", {name: "Show more"})
        await userEvent.click(expand)
        expect(expand).toHaveAttribute("aria-expanded", "true")
        const details = canvas.getByRole("group", {name: "Message"})
        expect(details).toHaveTextContent("End of notification.")
        const text = within(details).getByText(/End of notification\./)
        expect(text.textContent).toBe(FULL_TEXT)
        expect(getComputedStyle(text).whiteSpace).toBe("pre-wrap")
        if (window.innerWidth < 600) {
            const bounds = text.getBoundingClientRect()
            expect(bounds.left).toBeGreaterThanOrEqual(0)
            expect(bounds.right).toBeLessThanOrEqual(window.innerWidth)
        }
        await userEvent.click(within(row).getByRole("button", {name: "Show less"}))
        expect(canvas.queryByRole("group", {name: "Message"})).toBeNull()
    },
}

export const ExpandMessagePhone: Story = {
    ...ExpandMessage,
    globals: {viewport: {value: "mobile1", isRotated: false}},
}

export const ExpandEmailAndTemplate: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const rows = canvas.getAllByRole("row")
        const email = rows.find((row) => row.textContent?.includes(EMAIL_VOTER))
        const whatsapp = rows.find((row) => row.textContent?.includes(WHATSAPP_CODE))
        if (!email || !whatsapp) throw new Error("Email or WhatsApp row is missing")
        await userEvent.click(within(email).getByRole("button", {name: "Show more"}))
        expect(await canvas.findByTitle("Email content")).toHaveAttribute("sandbox", "")
        await userEvent.click(within(whatsapp).getByRole("button", {name: "Show more"}))
        const details = canvas.getAllByRole("group", {name: "Message"})
        expect(within(details[1]).getByText(WHATSAPP_CODE, {exact: true})).toBeVisible()
        expect(details[1]).toHaveTextContent("Template step_code (en)")
    },
}

export const WhatsApp: Story = {
    args: {mode: EMessageChannel.WHATSAPP},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("button", {name: "WhatsApp"})).toHaveAttribute(
            "aria-pressed",
            "true"
        )
        await userEvent.click(canvas.getByRole("button", {name: chatButton(WHATSAPP_VOTER)}))
        await expect(canvas.getByText(WHATSAPP_CODE)).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Copy code"})).toBeVisible()
    },
}

export const Viber: Story = {
    args: {mode: EMessageChannel.VIBER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: chatButton(VIBER_VOTER)}))
        await expect(canvas.getByText("Template tpl-otp-en (en)")).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Copy code"})).toBeNull()
    },
}

export const Messenger: Story = {
    args: {mode: EMessageChannel.MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("form", {name: "Get Started"})).toBeVisible()
        await expect(canvas.getByRole("button", {name: chatButton(MESSENGER_VOTER)})).toBeVisible()
    },
}

export const Sms: Story = {
    args: {mode: EMessageChannel.SMS},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: chatButton(SMS_VOTER)}))
        await expect(
            canvas.getByText("120934 is your verification code for Council.")
        ).toBeVisible()
        await expect(canvas.getByText("No delivery reports on this channel")).toBeVisible()
    },
}

export const Email: Story = {
    args: {mode: EMessageChannel.EMAIL},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("list", {name: "Inbox"})).toBeVisible()
        const frame = await canvas.findByTitle("Email content")
        await expect(frame).toHaveAttribute("sandbox", "")
        await expect(canvas.getByText(`To: ${EMAIL_VOTER}`)).toBeVisible()
    },
}

export const Empty: Story = {
    args: {messages: []},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No messages yet.")).toBeVisible()
    },
}

export const RefreshFailed: Story = {
    args: {refreshFailed: true},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(
                "The messages could not be refreshed. The list shown is the last one read."
            )
        ).toBeVisible()
    },
}

export const FluidDescription: Story = {
    play: async ({canvasElement}) => {
        const intro = await within(canvasElement).findByText(
            /Messages the demo provider of this development stack received/
        )
        const container = intro.parentElement as HTMLElement
        const originalWidth = container.style.width
        try {
            for (const width of [1200, 600]) {
                container.style.width = `${width}px`
                await waitFor(() => {
                    expect(intro.getBoundingClientRect().width).toBeCloseTo(
                        container.getBoundingClientRect().width,
                        0
                    )
                })
            }
        } finally {
            container.style.width = originalWidth
        }
    },
}
