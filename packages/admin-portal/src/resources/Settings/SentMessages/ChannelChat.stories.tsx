// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, spyOn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EMessageChannel} from "@/types/messaging"
import {ChannelChat, ChatChannel} from "./ChannelChat"
import {IFakeGetStartedAnswer} from "./types"
import {
    MESSENGER_LINK,
    MESSENGER_VOTER,
    VIBER_PIN,
    VIBER_VOTER,
    WHATSAPP_CODE,
    WHATSAPP_VOTER,
    channelMessages,
    chatButton,
} from "./__stories__/SentMessagesFixture"

enum EGetStarted {
    ACCEPTED = "ACCEPTED",
    REFUSED = "REFUSED",
    FAILED = "FAILED",
}

interface Scenario {
    channel: ChatChannel
    empty: boolean
    /** A second WhatsApp number also writes to the first voter. */
    secondSender: boolean
    getStarted: EGetStarted
    clipboard: "granted" | "denied"
}

const ANSWERS: Record<EGetStarted, IFakeGetStartedAnswer> = {
    [EGetStarted.ACCEPTED]: {status: 200, reason: null},
    [EGetStarted.REFUSED]: {status: 403, reason: "Invalid signature"},
    [EGetStarted.FAILED]: {
        status: null,
        reason: "No Messenger account of this organization uses this Page",
    },
}

const SECOND_NUMBER = "209876543210"

/** A message to the first WhatsApp voter from another business number. */
const fromSecondNumber = () => ({
    ...channelMessages(EMessageChannel.WHATSAPP)[0],
    id: "fake-second-number",
    received_at: "2026-10-10T09:09:00Z",
    sender: SECOND_NUMBER,
})

let graphql: ReturnType<typeof graphqlBoundary>
let copied: string[] = []
const onGetStarted = fn<(link: string) => Promise<IFakeGetStartedAnswer>>()

const meta = {
    title: "Admin/Settings/Sent messages/ChannelChat",
    component: ChannelChat,
    args: {
        channel: EMessageChannel.WHATSAPP,
        empty: false,
        secondSender: false,
        getStarted: EGetStarted.ACCEPTED,
        clipboard: "granted",
    },
    argTypes: {
        channel: {
            control: "inline-radio",
            options: [
                EMessageChannel.WHATSAPP,
                EMessageChannel.VIBER,
                EMessageChannel.MESSENGER,
                EMessageChannel.SMS,
            ],
        },
        getStarted: {control: "inline-radio", options: Object.values(EGetStarted)},
        clipboard: {control: "inline-radio", options: ["granted", "denied"]},
    },
    beforeEach: ({args}) => {
        graphql = graphqlBoundary({})
        copied = []
        onGetStarted.mockReset()
        onGetStarted.mockImplementation(async () => ANSWERS[args.getStarted])
        const clipboard = spyOn(navigator.clipboard, "writeText").mockImplementation(
            async (text) => {
                if (args.clipboard === "denied") throw new DOMException("Denied", "NotAllowedError")
                copied.push(text)
            }
        )
        return () => clipboard.mockRestore()
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql}>
            <ChannelChat
                channel={args.channel}
                messages={
                    args.empty
                        ? []
                        : [
                              ...channelMessages(args.channel),
                              ...(args.secondSender ? [fromSecondNumber()] : []),
                          ]
                }
                onGetStarted={onGetStarted}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const WhatsAppCode: Story = {
    parameters: {
        widgets: [
            "PhoneFrame",
            "ChatHeader",
            "ConversationList",
            "ChatBubble",
            "TemplateBody",
            "CopyCodeButton",
        ],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("region", {name: "WhatsApp on a phone"})).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: chatButton(WHATSAPP_VOTER)}))
        // Oldest first, as a chat reads; each with its report.
        const chat = within(canvas.getByRole("list", {name: `Messages to ${WHATSAPP_VOTER}`}))
        const bubbles = chat.getAllByRole("listitem")
        await expect(bubbles).toHaveLength(2)
        await expect(within(bubbles[0]).getByText(WHATSAPP_CODE)).toBeVisible()
        await expect(
            within(bubbles[0]).getByRole("img", {
                name: "Report accepted by the webhook component (HTTP 200)",
            })
        ).toBeVisible()
        await expect(within(bubbles[0]).getByText(/is your verification code/)).toBeVisible()
        await expect(within(bubbles[0]).getByText(/do not share/i)).toBeVisible()
        await expect(within(bubbles[1]).getByText(/Voting reminder/)).toBeVisible()
        await expect(within(bubbles[1]).getByText(/Council Election 2026/)).toBeVisible()
        await expect(within(bubbles[1]).getByText(/Hello Ana/)).toBeVisible()
        await expect(within(bubbles[1]).getByText("Report refused: HTTP 401")).toBeVisible()
        const bubble = bubbles[0].firstElementChild as HTMLElement
        const copyButton = within(bubbles[0]).getByRole("button", {name: "Copy code"})
        // Template messages use the chat width; the copy action fills their footer.
        expect(bubble.getBoundingClientRect().width).toBeGreaterThan(
            bubbles[0].getBoundingClientRect().width * 0.8
        )
        expect(copyButton.getBoundingClientRect().width).toBeGreaterThan(
            bubble.getBoundingClientRect().width * 0.95
        )
        expect(getComputedStyle(copyButton).backgroundColor).toBe("rgba(0, 0, 0, 0)")
        copyButton.focus()
        expect(getComputedStyle(copyButton).backgroundColor).toBe("rgba(0, 0, 0, 0)")
        await userEvent.click(copyButton)
        await expect(await canvas.findByText("Code copied")).toBeVisible()
        expect(copied).toEqual([WHATSAPP_CODE])
        await userEvent.click(canvas.getByRole("button", {name: "Back to the chats"}))
        await expect(canvas.getByText("WhatsApp chats")).toBeVisible()
    },
}

export const WhatsAppCodeOnPhone: Story = {
    ...WhatsAppCode,
    globals: {viewport: {value: "mobile1", isRotated: false}},
}

export const CopyDenied: Story = {
    args: {clipboard: "denied"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: chatButton(WHATSAPP_VOTER)}))
        await userEvent.click(canvas.getByRole("button", {name: "Copy code"}))
        await expect(await canvas.findByText("The code could not be copied")).toBeVisible()
    },
}

export const ViberCode: Story = {
    args: {channel: EMessageChannel.VIBER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: chatButton(VIBER_VOTER)}))
        await expect(canvas.getByText(VIBER_PIN)).toBeVisible()
        await expect(canvas.getByText(/Voting reminder/)).toBeVisible()
        await expect(canvas.getByText(/Council Election 2026/)).toBeVisible()
        await expect(
            canvas.getByText("The report got no answer: The webhook component didn't answer")
        ).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Copy code"})).toBeNull()
    },
}

export const MessengerGetStarted: Story = {
    args: {channel: EMessageChannel.MESSENGER},
    parameters: {widgets: ["GetStartedBox"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const form = within(canvas.getByRole("form", {name: "Get Started"}))
        const submit = form.getByRole("button", {name: "Get Started"})
        await expect(submit).toBeDisabled()
        await userEvent.type(form.getByRole("textbox", {name: "Messenger link"}), MESSENGER_LINK)
        await userEvent.click(submit)
        await waitFor(() => expect(onGetStarted).toHaveBeenCalledWith(MESSENGER_LINK))
        await expect(
            await form.findByText(
                "The webhook component answered 200. The code arrives in the chat."
            )
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: chatButton(MESSENGER_VOTER)}))
        await expect(canvas.getByText(/739201 is your verification code/)).toBeVisible()
    },
}

export const GetStartedRefused: Story = {
    args: {channel: EMessageChannel.MESSENGER, getStarted: EGetStarted.REFUSED},
    play: async ({canvasElement}) => {
        const form = within(within(canvasElement).getByRole("form", {name: "Get Started"}))
        await userEvent.type(form.getByRole("textbox", {name: "Messenger link"}), MESSENGER_LINK)
        await userEvent.click(form.getByRole("button", {name: "Get Started"}))
        await expect(
            await form.findByText("The webhook component answered 403. Invalid signature")
        ).toBeVisible()
    },
}

export const GetStartedFailed: Story = {
    args: {channel: EMessageChannel.MESSENGER, getStarted: EGetStarted.FAILED},
    play: async ({canvasElement}) => {
        const form = within(within(canvasElement).getByRole("form", {name: "Get Started"}))
        await userEvent.type(
            form.getByRole("textbox", {name: "Messenger link"}),
            "https://m.me/other.page?ref=abc"
        )
        await userEvent.click(form.getByRole("button", {name: "Get Started"}))
        await expect(
            await form.findByText(
                "Get Started failed: No Messenger account of this organization uses this Page"
            )
        ).toBeVisible()
    },
}

export const SmsChat: Story = {
    args: {channel: EMessageChannel.SMS},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("region", {name: "SMS on a phone"})).toBeVisible()
        expect(canvas.queryByRole("form", {name: "Get Started"})).toBeNull()
    },
}

export const NoMessages: Story = {
    args: {channel: EMessageChannel.VIBER, empty: true},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No Viber messages yet.")).toBeVisible()
    },
}

export const OneChatPerSender: Story = {
    args: {secondSender: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // The same voter, two business numbers: two chats, each naming its sender (R6-07).
        const chats = canvas.getAllByRole("button", {name: chatButton(WHATSAPP_VOTER)})
        await expect(chats).toHaveLength(2)
        await userEvent.click(
            canvas.getByRole("button", {
                name: `Open the chat with ${WHATSAPP_VOTER} from ${SECOND_NUMBER}`,
            })
        )
        const chat = within(canvas.getByRole("list", {name: `Messages to ${WHATSAPP_VOTER}`}))
        await expect(chat.getAllByRole("listitem")).toHaveLength(1)
        await expect(canvas.getByText(`From ${SECOND_NUMBER}`)).toBeVisible()
    },
}
