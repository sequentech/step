// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {eventRecord} from "@/__stories__/fixtures"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, fn, userEvent, waitFor, within} from "storybook/test"
import type {Identifier} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, type StoryRecord, storyId} from "@/__stories__/fixtures"
import type {Sequent_Backend_Template} from "@/gql/graphql"
import {ITemplateMethod} from "@/types/templates"
import {
    EChannelSelection,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    MESSAGING_CONFIG_ANNOTATION,
} from "@/types/messaging"
import {AudienceSelection, SendTemplate} from "./SendTemplate"

interface Scenario {
    ids?: Identifier[]
    audienceSelection?: AudienceSelection
    secretAttributeNames?: string[]
    failure: boolean
    close: () => void
}

const welcome = {
    subject: "Welcome to the election",
    plaintext_body: "Your reference is {{user.security-answer}}",
    html_body: "<p>Your reference is {{user.security-answer}}</p>",
}

const template = (
    index: number,
    communication_method: ITemplateMethod,
    alias: string,
    body: object
): StoryRecord<Sequent_Backend_Template> => ({
    id: storyId(9, index),
    tenant_id: TENANT_ID,
    alias,
    communication_method,
    type: "CREDENTIALS",
    template: {alias, ...body},
    annotations: {},
    labels: {},
    created_by: "admin",
    created_at: "2026-01-01T00:00:00.000Z",
    updated_at: "2026-01-01T00:00:00.000Z",
})

const whatsappReminder = {
    message: "Hello {{1}}, voting opens tomorrow.",
    parameters: ["user.first_name"],
}

const templates = [
    template(1, ITemplateMethod.EMAIL, "welcome", {email: welcome}),
    template(2, ITemplateMethod.SMS, "reminder", {sms: {message: "Remember to vote"}}),
    template(3, ITemplateMethod.EMAIL, "voting-opens", {
        selected_methods: {EMAIL: true, SMS: true, WHATSAPP: true},
        email: welcome,
        sms: {message: "Voting opens tomorrow"},
        whatsapp: whatsappReminder,
    }),
]

const SMS_ACCOUNT_ID = storyId(9, 20)
const EMAIL_ACCOUNT_ID = storyId(9, 21)

const account = (
    id: string,
    channel: EMessageChannel,
    provider: EMessagingProvider,
    name: string
) => ({
    id,
    tenant_id: TENANT_ID,
    channel,
    provider,
    name,
    sender: {provider},
    credentials: {},
    limits: {allowed_calling_codes: []},
    provider_approval: "CONFIRMED",
    status: {connected: true, production_access: true, approved_templates: {}},
    webhook_key: null,
    is_default: true,
    created_at: "2026-01-01T00:00:00.000Z",
    updated_at: "2026-01-01T00:00:00.000Z",
})

const accounts = [
    account(
        EMAIL_ACCOUNT_ID,
        EMessageChannel.EMAIL,
        EMessagingProvider.AWS_SES,
        "Election office email"
    ),
    account(SMS_ACCOUNT_ID, EMessageChannel.SMS, EMessagingProvider.AWS_SNS, "Election office SMS"),
]

const messagingConfig = {
    version: 1,
    channels: [
        {
            channel: EMessageChannel.EMAIL,
            account_id: EMAIL_ACCOUNT_ID,
            purposes: [EMessagePurpose.OTP, EMessagePurpose.NOTICE],
            templates: [],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
        {
            channel: EMessageChannel.SMS,
            account_id: SMS_ACCOUNT_ID,
            purposes: [EMessagePurpose.NOTICE],
            templates: [],
            out_of_window: EOutOfWindowPolicy.DISABLED,
        },
    ],
    notice_fallback: [EMessageChannel.SMS],
    election_channels: {},
    reply_text: {},
}

const electionEvent = eventRecord(undefined, {
    annotations: {[MESSAGING_CONFIG_ANNOTATION]: JSON.stringify(messagingConfig)},
})

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/User/SendTemplate",
    component: SendTemplate,
    args: {ids: [STORY_IDS.user, STORY_IDS.secondUser], failure: false, close: fn()},
    parameters: {
        router: {initialEntries: ["/sequent_backend_election_event/voters"]},
        expectedFailure: {
            reason: "The audience, method and alias selects have no accessible name, and the three section regions share one.",
            a11y: ["aria-input-field-name", "landmark-unique"],
        },
    },
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                GetMessagingAccounts: () => ({
                    data: {sequent_backend_messaging_account: accounts},
                }),
                CreateScheduledEvent: () => {
                    if (args.failure) throw new Error("Synthetic scheduler failure")
                    return {data: {createScheduledEvent: {id: storyId(9, 9)}}}
                },
            },
            {schema: false}
        )
        // The event's zones: the schedule is entered in its primary timezone.
        data = resourceBoundary({
            sequent_backend_template: templates,
            sequent_backend_election_event: [electionEvent],
            sequent_backend_election: [],
        })
        await graphql.ready
    },
    render: ({ids, audienceSelection, secretAttributeNames, close}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <SendTemplate
                ids={ids}
                audienceSelection={audienceSelection}
                electionEventId={EVENT_ID}
                secretAttributeNames={secretAttributeNames}
                close={close}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const defaultEmail = {
    subject: "Participate in {{election_event.name}}",
    plaintext_body: "Hello {{user.first_name}},\n\nEnter in {{vote_url}} to vote",
    html_body: "<p>Hello {{user.first_name}},<br><br>Enter in {{vote_url}} to vote</p>",
}

const sent = () => graphql.calls.find((call) => call.name === "CreateScheduledEvent")?.variables

async function send(canvasElement: HTMLElement) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Send Notification"}))
}

async function choose(canvasElement: HTMLElement, select: string, option: string) {
    const [combobox] = within(canvasElement)
        .getAllByRole("combobox")
        .filter((element) => element.textContent === select)
    await userEvent.click(combobox)
    await userEvent.click(await within(document.body).findByRole("option", {name: option}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Send a notification to voters.")).toBeVisible()
        expect(canvas.getByText("To 2 Selected voters")).toBeVisible()
        expect(canvas.getByRole("switch", {name: "Send now"})).toBeChecked()
        expect(canvas.getByRole("textbox", {name: "Email Subject"})).toHaveValue(
            defaultEmail.subject
        )
        await waitFor(() =>
            expect(data.calls.map(({args}) => args)).toContainEqual([
                "sequent_backend_template",
                expect.objectContaining({filter: {tenant_id: TENANT_ID}}),
            ])
        )
        expect(canvas.getByText("Each voter's channel")).toBeVisible()
        await expect(await canvas.findByRole("cell", {name: "Election office SMS"})).toBeVisible()
        expect(graphql.calls.map((call) => call.name)).toEqual(["GetMessagingAccounts"])
    },
}

export const SendNowToTheSelectedVoters: Story = {
    play: async ({args, canvasElement}) => {
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(sent()).toEqual({
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
            eventProcessor: "SEND_TEMPLATE",
            eventPayload: {
                audience_selection: AudienceSelection.SELECTED,
                audience_voter_ids: [STORY_IDS.user, STORY_IDS.secondUser],
                channel_selection: EChannelSelection.VOTER_PREFERENCE,
                schedule_now: true,
                email: defaultEmail,
                sms: {message: "Enter in {{vote_url}} to vote"},
                secret_attribute_names: [],
            },
        })
        const notice = await within(document.body).findByText(
            "Notification programmed/sent successfully"
        )
        await waitFor(() => expect(notice).toBeVisible())
    },
}

/** The existing notification payload retains the entered time without changing dispatch. */
export const ScheduledTimeKeepsItsZone: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("switch", {name: "Send now"}))
        fireEvent.change(canvas.getByLabelText("Date and time to start sending notifications"), {
            target: {value: "2028-04-09T08:00"},
        })
        await send(canvasElement)
        await waitFor(() =>
            expect(sent()?.eventPayload).toEqual(
                expect.objectContaining({
                    schedule_now: false,
                    schedule_date: "2028-04-09T08:00:00Z",
                    schedule_local: "2028-04-09T08:00",
                    schedule_timezone: "UTC",
                })
            )
        )
        expect(sent()?.cronConfig).toBeUndefined()
    },
}

export const SendToVotersWhoHaveNotVoted: Story = {
    args: {ids: undefined},
    play: async ({args, canvasElement}) => {
        await choose(canvasElement, "To 0 Selected voters", "Those who didn't vote yet")
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(sent()?.eventPayload).toMatchObject({
            audience_selection: AudienceSelection.NOT_VOTED,
        })
    },
}

export const EmailTemplateWithSecretAttribute: Story = {
    args: {
        audienceSelection: AudienceSelection.ALL_USERS,
        secretAttributeNames: ["security-answer"],
    },
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() => expect(data.calls).not.toHaveLength(0))
        // The template is the third select (the schedule's timezone picker isn't one).
        const [, , alias] = canvas.getAllByRole("combobox").filter((box) => box.tagName !== "INPUT")
        await userEvent.click(alias)
        await userEvent.click(await within(document.body).findByRole("option", {name: "welcome"}))
        await waitFor(() =>
            expect(canvas.getByRole("textbox", {name: "Email Subject"})).toHaveValue(
                welcome.subject
            )
        )
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        // Only the secret attributes the template uses are decrypted for it.
        expect(sent()?.eventPayload).toMatchObject({
            audience_selection: AudienceSelection.ALL_USERS,
            email: welcome,
            secret_attribute_names: ["security-answer"],
        })
    },
}

export const SmsMessage: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await choose(canvasElement, "Each voter's channel", "SMS only")
        const message = await canvas.findByRole("textbox", {name: "SMS Message"})
        await userEvent.clear(message)
        await userEvent.type(message, "Polls close at 20:00")
        expect(canvas.queryByRole("textbox", {name: "Email Subject"})).toBeNull()
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(sent()?.eventPayload).toMatchObject({
            channel_selection: EChannelSelection.SINGLE_CHANNEL,
            communication_method: ITemplateMethod.SMS,
            sms: {message: "Polls close at 20:00"},
        })
        expect(sent()?.eventPayload).not.toHaveProperty("email")
    },
}

export const MultiMethodTemplateForSms: Story = {
    // The open listbox hides the unnamed selects from the accessibility tree.
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await choose(canvasElement, "Each voter's channel", "SMS only")
        const [, , alias] = within(canvasElement).getAllByRole("combobox")
        await userEvent.click(alias)
        const options = within(await within(document.body).findByRole("listbox"))
            .getAllByRole("option")
            .map((option) => option.textContent)
        expect(options).toEqual(["voting-opens", "reminder"])
    },
}

export const WhatsAppOnly: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await choose(canvasElement, "Each voter's channel", "WhatsApp only")
        const [, , alias] = canvas.getAllByRole("combobox")
        await userEvent.click(alias)
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "voting-opens"})
        )
        await expect(await canvas.findByRole("textbox", {name: "WhatsApp"})).toHaveValue(
            whatsappReminder.message
        )
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(sent()?.eventPayload).toMatchObject({
            channel_selection: EChannelSelection.SINGLE_CHANNEL,
            communication_method: ITemplateMethod.WHATSAPP,
            alias: "voting-opens",
            whatsapp: whatsappReminder,
        })
        expect(sent()?.eventPayload.whatsapp).toEqual(whatsappReminder)
    },
}

export const WhatsAppWithItsProviderTemplate: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await choose(canvasElement, "Each voter's channel", "WhatsApp only")
        const [, , alias] = canvas.getAllByRole("combobox")
        await userEvent.click(alias)
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "voting-opens"})
        )
        await expect(
            canvas.getByText(/the event's template bound to the chosen template's alias is used/)
        ).toBeVisible()
        await userEvent.type(
            await canvas.findByRole("textbox", {name: "WhatsApp provider template"}),
            "voting_opens"
        )
        await userEvent.type(
            canvas.getByRole("textbox", {name: "WhatsApp provider language"}),
            "en_US"
        )
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(sent()?.eventPayload.whatsapp).toEqual({
            ...whatsappReminder,
            provider_template: "voting_opens",
            provider_language: "en_US",
        })
    },
}

export const MissingChannelContent: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const smsMessage = await canvas.findByRole("textbox", {name: "SMS Message"})
        await userEvent.clear(smsMessage)
        await expect(await canvas.findByText(/has no content for SMS/)).toBeVisible()
    },
}

export const ScheduleNeedsADate: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("switch", {name: "Send now"}))
        expect(canvas.getByLabelText(/Date and time to start sending/)).toBeEnabled()
        await send(canvasElement)
        await expect(await canvas.findByText("Please choose a date")).toBeVisible()
        expect(graphql.calls.map((call) => call.name)).toEqual(["GetMessagingAccounts"])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const SendFailure: Story = {
    args: {failure: true},
    play: async ({args, canvasElement}) => {
        await send(canvasElement)
        await expect(
            await within(canvasElement).findByText(/Error sending the notification/)
        ).toBeVisible()
        expect(graphql.calls.map((call) => call.name)).toEqual([
            "GetMessagingAccounts",
            "CreateScheduledEvent",
        ])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const SelectedUsers: Story = {
    parameters: {router: {initialEntries: ["/user-roles/users"]}},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("To 2 Selected users")).toBeVisible()
    },
}
