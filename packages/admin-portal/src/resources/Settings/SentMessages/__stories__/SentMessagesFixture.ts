// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessageChannel, EMessagingProvider} from "@/types/messaging"
import {EFakeReportState, IFakeMessageContent, IFakeProviderMessage, IFakeReport} from "../types"

export const WHATSAPP_VOTER = "+34600000001"
export const WHATSAPP_CODE = "482913"
export const VIBER_VOTER = "34600000003"
export const VIBER_PIN = "551204"
export const MESSENGER_VOTER = "6543210987654321"
export const SMS_VOTER = "+639171234567"
export const EMAIL_VOTER = "ana@example.org"
export const MESSENGER_LINK = "https://m.me/council.page?ref=Q2xKmB8vR3"

// Every field present, as the action answers.
const content = (fields: Partial<IFakeMessageContent>): IFakeMessageContent => ({
    text: null,
    subject: null,
    html: null,
    template_name: null,
    template_language: null,
    parameters: [],
    code: null,
    truncated: false,
    ...fields,
})

const report = (fields: Partial<IFakeReport> & Pick<IFakeReport, "state">): IFakeReport => ({
    status: null,
    reason: null,
    ...fields,
})

const accepted = report({state: EFakeReportState.ANSWERED, status: 200})
const noReceipts = report({
    state: EFakeReportState.NO_RECEIPTS,
    reason: "This provider sends no delivery reports",
})

/**
 * What the fake provider received, in full (as `messaging_fake_provider_message`
 * answers one entry): each channel, and each report outcome.
 */
export const fullMessages = (): IFakeProviderMessage[] => [
    {
        id: "fake-1",
        received_at: "2026-10-10T09:00:00Z",
        channel: EMessageChannel.WHATSAPP,
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        sender: "109876543210",
        recipient: WHATSAPP_VOTER,
        content: content({
            template_name: "step_code",
            template_language: "en",
            parameters: [{name: null, value: WHATSAPP_CODE}],
            code: WHATSAPP_CODE,
        }),
        report: accepted,
    },
    {
        id: "fake-2",
        received_at: "2026-10-10T09:05:00Z",
        channel: EMessageChannel.WHATSAPP,
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        sender: "109876543210",
        recipient: WHATSAPP_VOTER,
        content: content({
            template_name: "step_notice",
            template_language: "en",
            parameters: [
                {name: null, value: "Ana"},
                {name: null, value: "Council Election 2026"},
                {name: null, value: "https://vote.example.org/council"},
            ],
        }),
        // The component refused the report: visible, not silent.
        report: report({state: EFakeReportState.ANSWERED, status: 401}),
    },
    {
        id: "fake-3",
        received_at: "2026-10-10T09:06:00Z",
        channel: EMessageChannel.WHATSAPP,
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        sender: "109876543210",
        recipient: "+34600000002",
        content: content({
            template_name: "step_code",
            template_language: "es",
            parameters: [{name: null, value: "300177"}],
            code: "300177",
        }),
        report: report({state: EFakeReportState.PENDING}),
    },
    {
        id: "fake-4",
        received_at: "2026-10-10T09:02:00Z",
        channel: EMessageChannel.VIBER,
        provider: EMessagingProvider.VIBER_INFOBIP,
        sender: "Council",
        recipient: VIBER_VOTER,
        content: content({
            template_name: "step_code",
            template_language: "en",
            parameters: [{name: "pin", value: VIBER_PIN}],
            code: VIBER_PIN,
        }),
        report: accepted,
    },
    {
        id: "fake-5",
        received_at: "2026-10-10T09:07:00Z",
        channel: EMessageChannel.VIBER,
        provider: EMessagingProvider.VIBER_INFOBIP,
        sender: "Council",
        recipient: VIBER_VOTER,
        content: content({
            template_name: "step_notice",
            template_language: "en",
            parameters: [
                {name: null, value: "Ana"},
                {name: null, value: "Council Election 2026"},
                {name: null, value: "https://vote.example.org/council"},
            ],
        }),
        report: report({
            state: EFakeReportState.UNREACHABLE,
            reason: "The webhook component didn't answer",
        }),
    },
    {
        id: "fake-6",
        received_at: "2026-10-10T09:03:00Z",
        channel: EMessageChannel.MESSENGER,
        provider: EMessagingProvider.MESSENGER_SEND_API,
        sender: "1029384756",
        recipient: MESSENGER_VOTER,
        content: content({
            text: "739201 is your verification code for Council. It expires in 10 minutes.",
        }),
        report: accepted,
    },
    {
        id: "fake-7",
        received_at: "2026-10-10T09:04:00Z",
        channel: EMessageChannel.SMS,
        provider: EMessagingProvider.AWS_SNS,
        sender: "Council",
        recipient: SMS_VOTER,
        content: content({text: "120934 is your verification code for Council."}),
        report: noReceipts,
    },
    {
        id: "fake-8",
        received_at: "2026-10-10T09:01:00Z",
        channel: EMessageChannel.EMAIL,
        provider: EMessagingProvider.AWS_SES,
        sender: "no-reply@council.example.org",
        recipient: EMAIL_VOTER,
        content: content({
            subject: "Your enrollment was received",
            text: "Hello Ana,\n\nWe received your enrollment. We will write again once it is reviewed.",
            // A longer body would be cut at 64 KiB; the reading pane says so.
            truncated: true,
        }),
        report: noReceipts,
    },
    {
        id: "fake-9",
        received_at: "2026-10-10T09:08:00Z",
        channel: EMessageChannel.EMAIL,
        provider: EMessagingProvider.AWS_SES,
        sender: "no-reply@council.example.org",
        recipient: EMAIL_VOTER,
        content: content({
            subject: "Your verification code",
            text: "Your verification code is 845120.",
            // The script never runs: the reading pane's iframe is sandboxed.
            html: '<h1>Your verification code</h1><p>Use <b>845120</b> to continue.</p><script>document.body.textContent = "script ran"</script>',
        }),
        report: noReceipts,
    },
    {
        id: "fake-10",
        received_at: "2026-10-10T08:59:00Z",
        channel: EMessageChannel.MESSENGER,
        provider: EMessagingProvider.MESSENGER_SEND_API,
        sender: "5550001112",
        recipient: "1234567890123456",
        content: content({text: "Your enrollment was received."}),
        // Two accounts use that Page: the fake can't tell whose report it is.
        report: report({
            state: EFakeReportState.NOT_SENT,
            reason: "Several accounts use this sender",
        }),
    },
]

/** An entry as the list answers it: without the email HTML, with `has_html` (R6-03). */
const listEntry = (message: IFakeProviderMessage): IFakeProviderMessage => {
    const {html, ...content} = message.content
    return {...message, content: {...content, has_html: !!html}}
}

/** The list, as `messaging_fake_provider_messages` answers it. */
export const sentMessages = (): IFakeProviderMessage[] => fullMessages().map(listEntry)

/** One entry in full, as `messaging_fake_provider_message(id)` answers it. */
export const fullMessage = (id: string): IFakeProviderMessage | null =>
    fullMessages().find((message) => message.id === id) ?? null

/** The messages of one channel, as listed. */
export const channelMessages = (channel: EMessageChannel): IFakeProviderMessage[] =>
    sentMessages().filter((message) => message.channel === channel)

/** The stories' answer to `messaging_fake_provider_message(id)`. */
export const fakeMessageHandler = (operation: {variables: Record<string, unknown>}) => ({
    data: {messaging_fake_provider_message: fullMessage(String(operation.variables.id))},
})

/** The accessible name of a recipient's chat in the chat list, whatever its sender. */
export const chatButton = (recipient: string): RegExp =>
    new RegExp(`^Open the chat with ${recipient.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")} from `)
