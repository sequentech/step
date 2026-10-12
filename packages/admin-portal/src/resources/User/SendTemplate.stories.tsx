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
import {IPermissions} from "@/types/keycloak"
import {
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    IEventMessagingConfig,
} from "@/types/messaging"
import {ESendsFrom, IAudienceChannel} from "@/queries/GetMessagingAudiencePreview"
import {AudienceSelection, SendTemplate} from "./SendTemplate"

interface Scenario {
    ids?: Identifier[]
    audienceSelection?: AudienceSelection
    secretAttributeNames?: string[]
    failure: boolean
    close: () => void
    /** The event's stored configuration; `null` for an event without one. */
    config?: IEventMessagingConfig | null
    roles?: string[]
    /** The audience preview's rows, or `"FAIL"` when it can't be read. */
    preview?: IAudienceChannel[] | "FAIL"
    /** The saved WhatsApp template’s exact provider language. */
    providerLanguage?: string
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
    provider_template: "voting_opens",
    provider_language: "en_US",
}

const templates = [
    template(1, ITemplateMethod.EMAIL, "welcome", {email: welcome}),
    template(2, ITemplateMethod.SMS, "reminder", {sms: {message: "Remember to vote"}}),
    template(3, ITemplateMethod.EMAIL, "voting-opens", {
        selected_methods: {EMAIL: true, SMS: true, WHATSAPP: true, VIBER: true, MESSENGER: true},
        email: welcome,
        sms: {message: "Voting opens tomorrow"},
        whatsapp: whatsappReminder,
        viber: {message: "Approved Viber reminder", parameters: []},
        messenger: {message: "Voting opens tomorrow", parameters: []},
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

const messagingConfig: IEventMessagingConfig = {
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
    election_channels: {},
}

const electionEvent = eventRecord()

// Reads the event's messaging configuration and safe account columns.
const ROLES = [IPermissions.MESSAGING_CONFIG_READ]
const READS = ["GetEventMessagingConfig", "GetMessagingAccountSummaries"]
// The audience preview is read once the channels are known; asserted on its own.
const PREVIEW = "GetMessagingAudiencePreview"
const callNames = () =>
    graphql.calls
        .map((call) => call.name)
        .filter((name) => name !== PREVIEW)
        .sort()
const previewCalls = () => graphql.calls.filter((call) => call.name === PREVIEW)

// The two selected voters: one on SMS, one on Email.
const preview: IAudienceChannel[] = [
    {
        channel: EMessageChannel.SMS,
        voters: 1,
        not_offered: 0,
        sends_from: ESendsFrom.ACCOUNT,
        account_id: SMS_ACCOUNT_ID,
        account_name: "Election office SMS",
    },
    {
        channel: EMessageChannel.EMAIL,
        voters: 1,
        not_offered: 0,
        sends_from: ESendsFrom.ACCOUNT,
        account_id: EMAIL_ACCOUNT_ID,
        account_name: "Election office email",
    },
]

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
                GetMessagingAccountSummaries: () => ({
                    data: {sequent_backend_messaging_account: accounts},
                }),
                GetEventMessagingConfig: () => ({
                    data: {
                        sequent_backend_messaging_event_config:
                            args.config === null
                                ? []
                                : [
                                      {
                                          election_event_id: EVENT_ID,
                                          config: args.config ?? messagingConfig,
                                          version: 1,
                                      },
                                  ],
                    },
                }),
                GetMessagingAudiencePreview: () => {
                    if (args.preview === "FAIL") throw new Error("Synthetic preview failure")
                    const channels = args.preview ?? preview
                    return {
                        data: {
                            messaging_audience_preview: {
                                total: channels.reduce((sum, row) => sum + row.voters, 0),
                                will_send: channels
                                    .filter((row) => !!row.channel && !!row.sends_from)
                                    .filter((row) => row.sends_from !== ESendsFrom.NONE)
                                    .reduce((sum, row) => sum + row.voters - row.not_offered, 0),
                                channels,
                                // Selected voters are counted when asked.
                                status: {
                                    computed_at: "2026-10-09T08:00:00Z",
                                    pending: false,
                                    outcome: "COUNTED",
                                },
                            },
                        },
                    }
                },
                CreateScheduledEvent: () => {
                    if (args.failure) throw new Error("Synthetic scheduler failure")
                    return {data: {createScheduledEvent: {id: storyId(9, 9)}}}
                },
            },
            {schema: false}
        )
        // The event's zones: the schedule is entered in its primary timezone.
        data = resourceBoundary({
            sequent_backend_template: args.providerLanguage
                ? templates.map((entry, index) =>
                      index === 2
                          ? {
                                ...entry,
                                template: {
                                    ...entry.template,
                                    whatsapp: {
                                        ...whatsappReminder,
                                        provider_language: args.providerLanguage,
                                    },
                                },
                            }
                          : entry
                  )
                : templates,
            sequent_backend_election_event: [electionEvent],
            sequent_backend_election: [],
        })
        await graphql.ready
    },
    render: ({ids, audienceSelection, secretAttributeNames, close, roles}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles ?? ROLES}>
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

/** The template is the second select (the schedule's timezone picker isn't one). */
function aliasSelect(canvasElement: HTMLElement) {
    const [, alias] = within(canvasElement)
        .getAllByRole("combobox")
        .filter((box) => box.tagName !== "INPUT")
    return alias
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Send a notification to voters.")).toBeVisible()
        expect(canvas.getByText("To 2 Selected voters")).toBeVisible()
        expect(canvas.getByRole("switch", {name: "Send now"})).toBeChecked()
        await userEvent.click(canvas.getByRole("button", {name: "Email"}))
        expect(canvas.getByRole("textbox", {name: "Email Subject"})).toHaveValue(
            defaultEmail.subject
        )
        await waitFor(() =>
            expect(data.calls.map(({args}) => args)).toContainEqual([
                "sequent_backend_template",
                expect.objectContaining({filter: {tenant_id: TENANT_ID}}),
            ])
        )
        await expect(await canvas.findByRole("cell", {name: "Election office SMS"})).toBeVisible()
        const table = within(canvas.getByRole("table", {name: "Each voter's channel"}))
        expect(table.getAllByRole("columnheader").map((header) => header.textContent)).toEqual([
            "Channel",
            "Voters",
            "Sends from",
        ])
        // The voters of the send, counted on the server the way the send reads them.
        await expect(await canvas.findByText("Voters in this audience: 2")).toBeVisible()
        expect(previewCalls().map((call) => call.variables)).toContainEqual({
            electionEventId: EVENT_ID,
            audienceSelection: AudienceSelection.SELECTED,
            voterIds: [STORY_IDS.user, STORY_IDS.secondUser],
        })
        // Channel order: SMS before Email.
        expect(
            table
                .getAllByRole("row")
                .slice(1)
                .map((row) => row.getAttribute("data-channel"))
        ).toEqual([EMessageChannel.SMS, EMessageChannel.EMAIL])
        expect(canvas.getByText(/Each voter gets one message on their own channel/)).toBeVisible()
        // No channel override: every voter is sent on their own channel.
        expect(canvas.queryByText(/ only$/)).toBeNull()
        expect(callNames()).toEqual(READS)
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
                schedule_now: true,
                email: defaultEmail,
                sms: {message: "Enter in {{vote_url}} to vote"},
                secret_attribute_names: [],
            },
        })
        const notice = await within(document.body).findByText("Notification scheduled successfully")
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
        const alias = aliasSelect(canvasElement)
        await userEvent.click(alias)
        await userEvent.click(await within(document.body).findByRole("option", {name: "welcome"}))
        await userEvent.click(canvas.getByRole("button", {name: "Email"}))
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
        await userEvent.click(canvas.getByRole("button", {name: "SMS"}))
        const message = await canvas.findByRole("textbox", {name: "SMS Message"})
        await userEvent.clear(message)
        await userEvent.type(message, "Polls close at 20:00")
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(sent()?.eventPayload).toMatchObject({
            sms: {message: "Polls close at 20:00"},
            email: defaultEmail,
        })
        expect(sent()?.eventPayload).not.toHaveProperty("channel_selection")
        expect(sent()?.eventPayload).not.toHaveProperty("communication_method")
    },
}

export const TemplatesOfEveryChannel: Story = {
    // The open listbox hides the unnamed selects from the accessibility tree.
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const alias = aliasSelect(canvasElement)
        await userEvent.click(alias)
        const options = within(await within(document.body).findByRole("listbox"))
            .getAllByRole("option")
            .map((option) => option.textContent)
        // The templates list reads newest id first (react-admin's default sort).
        expect(options).toEqual(["voting-opens", "reminder", "welcome"])
    },
}

export const WhatsAppContentOfATemplate: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        const alias = aliasSelect(canvasElement)
        await userEvent.click(alias)
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "voting-opens"})
        )
        await userEvent.click(canvas.getByRole("button", {name: "WhatsApp"}))
        await expect(canvas.getByText(whatsappReminder.message)).toBeVisible()
        expect(canvas.queryByRole("textbox", {name: "WhatsApp"})).toBeNull()
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(sent()?.eventPayload).toMatchObject({
            alias: "voting-opens",
            whatsapp: whatsappReminder,
            sms: {message: "Voting opens tomorrow"},
        })
        expect(sent()?.eventPayload).not.toHaveProperty("communication_method")
    },
}

export const WhatsAppWithItsProviderTemplate: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        const alias = aliasSelect(canvasElement)
        await userEvent.click(alias)
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "voting-opens"})
        )
        await userEvent.click(canvas.getByRole("button", {name: "WhatsApp"}))
        const section = within(canvas.getByRole("region", {name: "WhatsApp"}))
        await expect(
            section.getByText(/approved by the provider.*cannot be changed here/)
        ).toBeVisible()
        await expect(section.getByText(whatsappReminder.message)).toBeVisible()
        expect(section.queryByRole("textbox")).toBeNull()
        expect(section.queryByText("WhatsApp provider template")).toBeNull()
        expect(section.queryByText("WhatsApp provider language")).toBeNull()
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(sent()?.eventPayload).toHaveProperty("whatsapp", {
            ...whatsappReminder,
            provider_template: "voting_opens",
            provider_language: args.providerLanguage ?? "en_US",
        })
    },
}

export const WhatsAppWithLanguageOnlyCode: Story = {
    ...WhatsAppWithItsProviderTemplate,
    args: {providerLanguage: "en"},
}

export const MissingChannelContent: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "SMS"}))
        const smsMessage = await canvas.findByRole("textbox", {name: "SMS Message"})
        await userEvent.clear(smsMessage)
        await expect(await canvas.findByText(/has no content for SMS/)).toBeVisible()
        await send(canvasElement)
        expect(sent()).toBeUndefined()
    },
}

export const WhatsAppAudienceWithoutContent: Story = {
    args: {
        config: {
            ...messagingConfig,
            channels: [
                ...messagingConfig.channels,
                {
                    channel: EMessageChannel.WHATSAPP,
                    account_id: storyId(9, 22),
                    purposes: [EMessagePurpose.NOTICE],
                    templates: [],
                    out_of_window: EOutOfWindowPolicy.DISABLED,
                },
            ],
        },
        preview: [
            {
                channel: EMessageChannel.WHATSAPP,
                voters: 1,
                not_offered: 0,
                sends_from: ESendsFrom.ACCOUNT,
                account_name: "Demo WhatsApp",
            },
        ],
    },
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(/has no content for WhatsApp/)).toBeVisible()
        await send(canvasElement)
        expect(sent()).toBeUndefined()
        expect(args.close).not.toHaveBeenCalled()
        await userEvent.click(aliasSelect(canvasElement))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "voting-opens"})
        )
        // Missing content on unused channels must not stop this WhatsApp-only audience.
        await userEvent.click(canvas.getByRole("button", {name: "SMS"}))
        await userEvent.clear(canvas.getByRole("textbox", {name: "SMS Message"}))
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(sent()?.eventPayload).toMatchObject({whatsapp: whatsappReminder})
    },
}

export const ScheduleNeedsADate: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("switch", {name: "Send now"}))
        expect(canvas.getByLabelText(/Date and time to start sending/)).toBeEnabled()
        await send(canvasElement)
        await expect(await canvas.findByText("Please choose a date")).toBeVisible()
        expect(callNames()).toEqual(READS)
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
        expect(callNames()).toEqual(["CreateScheduledEvent", ...READS])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const SelectedUsers: Story = {
    parameters: {router: {initialEntries: ["/user-roles/users"]}},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("To 2 Selected users")).toBeVisible()
    },
}

export const EventThatSendsNoNotices: Story = {
    args: {
        config: {
            ...messagingConfig,
            channels: messagingConfig.channels.map((channel) => ({
                ...channel,
                purposes: [EMessagePurpose.OTP],
            })),
        },
    },
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        // Never shown as SMS and Email: every voter's notice would be refused.
        await expect(await canvas.findByText(/turn on notices for no channel/)).toBeVisible()
        expect(canvas.queryByRole("table", {name: "Each voter's channel"})).toBeNull()
        await send(canvasElement)
        expect(callNames()).toEqual(READS)
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const EventWithoutAConfiguration: Story = {
    args: {config: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const table = within(await canvas.findByRole("table", {name: "Each voter's channel"}))
        expect(
            table
                .getAllByRole("row")
                .slice(1)
                .map((row) => row.getAttribute("data-channel"))
        ).toEqual([EMessageChannel.SMS, EMessageChannel.EMAIL])
    },
}

export const WithoutTheReadPermission: Story = {
    args: {roles: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(/needs the messaging-config-read permission/)
        ).toBeVisible()
        expect(canvas.queryByRole("table", {name: "Each voter's channel"})).toBeNull()
        expect(graphql.calls.filter((call) => READS.includes(call.name))).toEqual([])
    },
}

export const CollapsibleChannelsPreserveSendOverrides: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(aliasSelect(canvasElement))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "voting-opens"})
        )
        for (const name of ["Email", "SMS", "WhatsApp", "Viber", "Facebook Messenger"]) {
            await expect(canvas.getByRole("button", {name})).toHaveAttribute(
                "aria-expanded",
                "false"
            )
            await expect(
                within(canvas.getByRole("button", {name})).getByTestId("ExpandMoreIcon")
            ).toBeVisible()
        }
        expect(canvas.queryByRole("textbox", {name: "Email Subject"})).toBeNull()
        for (const [channel, field, value] of [
            ["Email", "Email Subject", "Polls close tonight"],
            ["SMS", "SMS Message", "SMS override for this send"],
            ["Facebook Messenger", "Facebook Messenger", "Messenger override for this send"],
        ]) {
            const toggle = canvas.getByRole("button", {name: channel})
            await userEvent.click(toggle)
            const input = await canvas.findByRole("textbox", {name: field})
            await userEvent.clear(input)
            await userEvent.type(input, value)
            await userEvent.click(toggle)
            await expect(toggle).toHaveAttribute("aria-expanded", "false")
            await userEvent.click(toggle)
            await expect(input).toHaveValue(value)
            await userEvent.click(toggle)
        }
        for (const channel of ["WhatsApp", "Viber"]) {
            await userEvent.click(canvas.getByRole("button", {name: channel}))
            const section = within(canvas.getByRole("region", {name: channel}))
            expect(section.queryByRole("textbox")).toBeNull()
            await expect(
                section.getByText(
                    channel === "WhatsApp" ? whatsappReminder.message : "Approved Viber reminder"
                )
            ).toBeVisible()
            await userEvent.click(canvas.getByRole("button", {name: channel}))
        }
        await send(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(sent()?.eventPayload).toMatchObject({
            alias: "voting-opens",
            email: {...welcome, subject: "Polls close tonight"},
            sms: {message: "SMS override for this send"},
            messenger: {message: "Messenger override for this send", parameters: []},
            whatsapp: whatsappReminder,
            viber: {message: "Approved Viber reminder", parameters: []},
        })
        // These are overrides of this send, not changes to the saved template.
        expect(templates[2].template).toMatchObject({
            email: welcome,
            sms: {message: "Voting opens tomorrow"},
            messenger: {message: "Voting opens tomorrow"},
        })
        expect(data.writes).toHaveLength(0)
    },
}
