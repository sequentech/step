/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    EProviderApproval,
    EReadinessBlocker,
    EReadinessPolicy,
    IChannelMessageStats,
    IEventMessagingConfig,
    IMessagingAccount,
} from "@/types/messaging"
import {emptyEventMessagingConfig} from "@/services/messaging"
import {IVoterCounts} from "@/queries/GetMessagingVoterCounts"
import {
    MessagingChannels,
    MessagingDeliveryStatus,
    MessagingTemplateBindings,
    MigratedConfigNotice,
} from "./EditElectionEventMessaging"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
    }),
}))
jest.mock("react-admin", () => ({
    useGetList: () => ({data: []}),
    useNotify: () => jest.fn(),
    useRecordContext: () => undefined,
    useRefresh: () => jest.fn(),
}))
jest.mock("react-router-dom", () => ({
    useLocation: () => ({pathname: "/sequent_backend_election_event/event-1", search: ""}),
    useNavigate: () => jest.fn(),
}))
jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useMutation: () => [jest.fn(), {loading: false}],
    useQuery: () => ({data: undefined, loading: false}),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        ChannelIcon: () => null,
        ChannelLabel: ({label}: {label: string}) => <span>{label}</span>,
    }),
    {virtual: true}
)
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-1"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true}),
}))
jest.mock("@/hooks/useMessagingAccounts", () => ({
    useMessagingAccounts: () => ({accounts: [], loading: false}),
}))
jest.mock("@/hooks/useEventMessagingConfig", () => ({
    useEventMessagingConfig: () => ({config: undefined, loading: false, refetch: jest.fn()}),
    useEventMessagingConfigAuthor: () => ({updatedBy: undefined, refetch: jest.fn()}),
}))
jest.mock("@/hooks/useZonedTime", () => ({
    useZonedTime: () => (value?: string | null) => (value ? `zoned ${value}` : ""),
}))
jest.mock("@/components/styles/ElectionHeaderStyles", () => ({
    ElectionHeaderStyles: {AccordionTitle: "span"},
}))

const account = (overrides: Partial<IMessagingAccount>): IMessagingAccount => ({
    id: "sms-1",
    tenant_id: "tenant-1",
    channel: EMessageChannel.SMS,
    provider: EMessagingProvider.AWS_SNS,
    name: "Transactional SMS",
    sender: {provider: EMessagingProvider.AWS_SNS, sender_id: "COUNCIL"},
    provider_approval: EProviderApproval.PENDING,
    status: {connected: true, production_access: true, approved_templates: {}},
    is_default: true,
    ...overrides,
})

const whatsapp = account({
    id: "wa-1",
    channel: EMessageChannel.WHATSAPP,
    provider: EMessagingProvider.WHATSAPP_CLOUD_API,
    name: "Council WhatsApp",
    sender: {
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        business_account_id: "1",
        phone_number_id: "2",
        display_phone_number: "+34 600 000 000",
        api_version: "v23.0",
    },
})

const configWith = (
    channels: Array<[EMessageChannel, string, EMessagePurpose[]]>
): IEventMessagingConfig => ({
    ...emptyEventMessagingConfig(),
    channels: channels.map(([channel, account_id, purposes]) => ({
        channel,
        account_id,
        purposes,
        templates: [],
        out_of_window:
            channel === EMessageChannel.MESSENGER
                ? EOutOfWindowPolicy.UTILITY_MESSAGES
                : EOutOfWindowPolicy.DISABLED,
    })),
})

const row = (channel: EMessageChannel) =>
    document.querySelector(`tr[data-channel="${channel}"]`) as HTMLElement

const purposeSwitch = (channel: EMessageChannel, purpose: EMessagePurpose) =>
    row(channel).querySelector(
        `input[aria-label*="messaging.purpose.${purpose}"]`
    ) as HTMLInputElement

describe("MessagingChannels", () => {
    it("keeps an unready purpose off and names what is missing", () => {
        const onChange = jest.fn()
        render(
            <MessagingChannels
                config={configWith([[EMessageChannel.WHATSAPP, "wa-1", []]])}
                accounts={[whatsapp]}
                errors={[]}
                canEdit
                onChange={onChange}
            />
        )
        expect(purposeSwitch(EMessageChannel.WHATSAPP, EMessagePurpose.OTP)).toBeDisabled()
        expect(
            within(row(EMessageChannel.WHATSAPP)).getAllByText(
                /messaging.blocker.NEEDS_PROVIDER_APPROVAL, messaging.blocker.NEEDS_APPROVED_TEMPLATE/
            )
        ).toHaveLength(2)
    })

    it("lets a ready purpose be switched on", () => {
        const onChange = jest.fn()
        render(
            <MessagingChannels
                config={configWith([[EMessageChannel.SMS, "sms-1", []]])}
                accounts={[account({})]}
                errors={[]}
                canEdit
                onChange={onChange}
            />
        )
        const notices = purposeSwitch(EMessageChannel.SMS, EMessagePurpose.NOTICE)
        expect(notices).toBeEnabled()
        fireEvent.click(notices)
        const next: IEventMessagingConfig = onChange.mock.calls[0][0]
        expect(next.channels[0].purposes).toEqual([EMessagePurpose.NOTICE])
        // No fallback order: a notice goes only to each voter's own channel.
        expect(Object.keys(next)).not.toContain("notice_fallback")
    })

    it("says to choose the account when the channel has accounts but none is chosen", () => {
        render(
            <MessagingChannels
                config={emptyEventMessagingConfig()}
                accounts={[account({})]}
                errors={[]}
                canEdit
                onChange={jest.fn()}
            />
        )
        expect(purposeSwitch(EMessageChannel.SMS, EMessagePurpose.OTP)).toBeDisabled()
        expect(
            within(row(EMessageChannel.SMS)).getAllByText("messagingEvent.chooseAccount")
        ).toHaveLength(2)
    })

    it("shows an account of a removed provider as unsupported, never ready", () => {
        render(
            <MessagingChannels
                config={configWith([[EMessageChannel.SMS, "console-1", []]])}
                accounts={[
                    account({
                        id: "console-1",
                        provider: EMessagingProvider.CONSOLE,
                        sender: {provider: EMessagingProvider.CONSOLE},
                    }),
                ]}
                errors={[]}
                canEdit
                onChange={jest.fn()}
            />
        )
        expect(purposeSwitch(EMessageChannel.SMS, EMessagePurpose.OTP)).toBeDisabled()
        expect(
            within(row(EMessageChannel.SMS)).getAllByText(/messaging.blocker.UNSUPPORTED_PROVIDER/)
        ).toHaveLength(2)
    })

    it("lists the channels in channel order", () => {
        render(
            <MessagingChannels
                config={emptyEventMessagingConfig()}
                accounts={[]}
                errors={[]}
                canEdit
                onChange={jest.fn()}
            />
        )
        expect(
            Array.from(document.querySelectorAll("tr[data-channel]")).map((tr) =>
                tr.getAttribute("data-channel")
            )
        ).toEqual(["WHATSAPP", "VIBER", "MESSENGER", "SMS", "EMAIL"])
    })

    it("asks for an account when a channel has none", () => {
        render(
            <MessagingChannels
                config={emptyEventMessagingConfig()}
                accounts={[]}
                errors={[]}
                canEdit
                onChange={jest.fn()}
            />
        )
        expect(
            within(row(EMessageChannel.VIBER)).getByText("messagingEvent.noAccount")
        ).toBeVisible()
        expect(purposeSwitch(EMessageChannel.VIBER, EMessagePurpose.OTP)).toBeDisabled()
    })

    it("shows the server's errors next to the purpose they are about", () => {
        render(
            <MessagingChannels
                config={configWith([[EMessageChannel.SMS, "sms-1", [EMessagePurpose.OTP]]])}
                accounts={[account({})]}
                errors={[
                    {
                        kind: "PURPOSE_NOT_READY",
                        channel: EMessageChannel.SMS,
                        purpose: EMessagePurpose.OTP,
                        blockers: [],
                    },
                ]}
                canEdit
                onChange={jest.fn()}
            />
        )
        expect(within(row(EMessageChannel.SMS)).getByRole("alert").textContent).toMatch(
            /^messagingEvent.error.PURPOSE_NOT_READY/
        )
    })

    it("is read only without the permission", () => {
        render(
            <MessagingChannels
                config={configWith([[EMessageChannel.SMS, "sms-1", [EMessagePurpose.OTP]]])}
                accounts={[account({})]}
                errors={[]}
                canEdit={false}
                onChange={jest.fn()}
            />
        )
        expect(purposeSwitch(EMessageChannel.SMS, EMessagePurpose.OTP)).toBeDisabled()
    })
})

const messenger = account({
    id: "page-1",
    channel: EMessageChannel.MESSENGER,
    provider: EMessagingProvider.MESSENGER_SEND_API,
    name: "Council Page",
    sender: {
        provider: EMessagingProvider.MESSENGER_SEND_API,
        page_id: "1234",
        api_version: "v23.0",
    },
})

describe("Messenger outside the 24-hour window", () => {
    const renderChannels = (config: IEventMessagingConfig, errors = []) => {
        render(
            <MessagingChannels
                config={config}
                accounts={[account({}), messenger, whatsapp]}
                errors={errors}
                canEdit
                onChange={jest.fn()}
            />
        )
    }

    it("uses the utility template, with no Do not send choice, on Messenger only", () => {
        renderChannels(
            configWith([
                [EMessageChannel.SMS, "sms-1", []],
                [EMessageChannel.WHATSAPP, "wa-1", []],
                [EMessageChannel.MESSENGER, "page-1", [EMessagePurpose.NOTICE]],
            ])
        )
        expect(
            within(row(EMessageChannel.MESSENGER)).getByText(/messagingEvent.outOfWindow.utility/)
        ).toBeVisible()
        expect(screen.queryByRole("combobox", {name: /outOfWindow/})).toBeNull()
        expect(
            within(row(EMessageChannel.SMS)).queryByText(/messagingEvent.outOfWindow.utility/)
        ).toBeNull()
        expect(
            within(row(EMessageChannel.WHATSAPP)).queryByText(/messagingEvent.outOfWindow.utility/)
        ).toBeNull()
    })

    it("shows the blocker under Messenger's notices when no utility template is bound", () => {
        renderChannels(
            configWith([[EMessageChannel.MESSENGER, "page-1", [EMessagePurpose.NOTICE]]]),
            [{kind: "UTILITY_TEMPLATE_MISSING", channel: EMessageChannel.MESSENGER}] as never
        )
        expect(within(row(EMessageChannel.MESSENGER)).getByRole("alert")).toHaveTextContent(
            "messagingEvent.error.UTILITY_TEMPLATE_MISSING"
        )
    })
})

describe("no readiness confirmed by an administrator", () => {
    it("keeps a stored ADMIN_CONFIRMED account to the provider's check", () => {
        render(
            <MessagingChannels
                config={configWith([[EMessageChannel.WHATSAPP, "wa-1", []]])}
                accounts={[
                    {
                        ...whatsapp,
                        status: null,
                        readiness: EReadinessPolicy.ADMIN_CONFIRMED,
                        provider_approval: EProviderApproval.CONFIRMED,
                    },
                ]}
                errors={[]}
                canEdit
                onChange={jest.fn()}
            />
        )
        expect(purposeSwitch(EMessageChannel.WHATSAPP, EMessagePurpose.OTP)).toBeDisabled()
    })
})

describe("MessagingTemplateBindings", () => {
    const bound = (): IEventMessagingConfig => {
        const config = configWith([
            [EMessageChannel.SMS, "sms-1", [EMessagePurpose.OTP]],
            [EMessageChannel.WHATSAPP, "wa-1", [EMessagePurpose.NOTICE]],
        ])
        config.channels[1].templates = [
            {
                purpose: EMessagePurpose.NOTICE,
                key: "reminder",
                language: "en",
                provider_template: "vote_reminder",
                provider_language: "en_US",
            },
        ]
        return config
    }
    const renderBindings = (
        config: IEventMessagingConfig,
        props: Partial<React.ComponentProps<typeof MessagingTemplateBindings>> = {}
    ) => {
        const onChange = jest.fn()
        render(
            <MessagingTemplateBindings
                config={config}
                accounts={[account({}), whatsapp, messenger]}
                languages={["en", "tl"]}
                templateAliases={["reminder"]}
                errors={[]}
                incomplete={[]}
                canEdit
                onChange={onChange}
                {...props}
            />
        )
        return onChange
    }
    const group = () => {
        const row = within(
            screen.getByRole("group", {name: /messagingEvent.templates.row.*"position":1/})
        )
        const toggle = row.getByRole("button", {name: /messagingEvent.templates.row/})
        if (toggle.getAttribute("aria-expanded") === "false") fireEvent.click(toggle)
        return row
    }

    it("edits the bindings of the channels that use approved templates", () => {
        const onChange = renderBindings(bound())
        expect(document.querySelector('[data-template-channel="SMS"]')).toBeNull()
        expect(group().getByLabelText("messagingEvent.column.key")).toHaveValue("reminder")
        expect(group().getByLabelText(/messagingEvent.column.template/)).toHaveValue(
            "vote_reminder"
        )
        expect(group().getByText("messagingEvent.templates.approval.NOT_APPROVED")).toBeVisible()
        fireEvent.change(group().getByLabelText("messagingEvent.column.providerLanguage"), {
            target: {value: "fil"},
        })
        const next: IEventMessagingConfig = onChange.mock.calls[0][0]
        expect(next.channels[1].templates[0].provider_language).toBe("fil")
    })

    it("adds a row for the channel's first purpose and language, and removes rows", () => {
        const onChange = renderBindings(bound())
        fireEvent.click(screen.getByRole("button", {name: /messagingEvent.templates.add/}))
        expect(onChange.mock.calls[0][0].channels[1].templates[1]).toEqual({
            purpose: EMessagePurpose.NOTICE,
            key: null,
            language: "en",
            provider_template: "",
            provider_language: null,
        })
        group()
        fireEvent.click(screen.getByRole("button", {name: /messagingEvent.templates.remove/}))
        expect(onChange.mock.calls[1][0].channels[1].templates).toEqual([])
    })

    it("explains the selection order and marks an incomplete row", () => {
        renderBindings(bound(), {
            incomplete: [{channel: EMessageChannel.WHATSAPP, index: 0}],
        })
        expect(screen.getByText("messagingEvent.templates.order")).toBeVisible()
        expect(group().getByRole("alert")).toHaveTextContent("messagingEvent.templates.incomplete")
    })

    it("offers Messenger its templates for utility messages", () => {
        renderBindings(configWith([[EMessageChannel.MESSENGER, "page-1", []]]))
        expect(screen.getByText(/messagingEvent.templates.noneOptional/)).toBeVisible()
    })

    it("says so when no channel uses templates", () => {
        renderBindings(configWith([[EMessageChannel.SMS, "sms-1", []]]))
        expect(screen.getByText("messagingEvent.templates.empty")).toBeVisible()
    })

    it("is read only without the permission", () => {
        renderBindings(bound(), {canEdit: false})
        expect(group().getByLabelText("messagingEvent.column.providerLanguage")).toBeDisabled()
        expect(screen.queryByRole("button", {name: /messagingEvent.templates.add/})).toBeNull()
        expect(screen.queryByRole("button", {name: /messagingEvent.templates.remove/})).toBeNull()
    })
})

describe("MessagingDeliveryStatus", () => {
    const stats = (overrides: Partial<IChannelMessageStats>): IChannelMessageStats => ({
        channel: EMessageChannel.SMS,
        sent_last_24h: 6,
        queued: 0,
        accepted: 4,
        delivered: 0,
        failed: 2,
        unknown: 1,
        unresolved: 3,
        ...overrides,
    })
    const config = configWith([
        [EMessageChannel.SMS, "sms-1", [EMessagePurpose.NOTICE]],
        [EMessageChannel.WHATSAPP, "wa-1", [EMessagePurpose.NOTICE]],
    ])

    it("shows delivery as unavailable for an account without receipts, never 0", () => {
        render(
            <MessagingDeliveryStatus
                config={config}
                accounts={[account({}), whatsapp]}
                stats={[
                    stats({}),
                    stats({
                        channel: EMessageChannel.WHATSAPP,
                        sent_last_24h: 8,
                        accepted: 2,
                        delivered: 5,
                        failed: 0,
                        unknown: 0,
                        unresolved: 0,
                    }),
                ]}
            />
        )
        const sms = within(row(EMessageChannel.SMS))
        expect(sms.getByText("messaging.deliveryUnavailable")).toBeVisible()
        expect(sms.getByText("4")).toBeVisible()
        const whatsappRow = within(row(EMessageChannel.WHATSAPP))
        expect(whatsappRow.queryByText("messaging.deliveryUnavailable")).toBeNull()
        expect(whatsappRow.getByText("5")).toBeVisible()
    })

    const counted = (
        channels: IVoterCounts["channels"],
        status: Partial<IVoterCounts["status"]> = {}
    ): IVoterCounts => ({
        total: channels.reduce((sum, entry) => sum + entry.voters, 0),
        channels,
        status: {
            computed_at: "2026-10-09T08:00:00Z",
            pending: false,
            outcome: "COUNTED",
            ...status,
        } as IVoterCounts["status"],
    })

    it("shows voters, the last 24 hours and Unresolved apart, in channel order", () => {
        render(
            <MessagingDeliveryStatus
                config={config}
                accounts={[account({}), whatsapp]}
                stats={[stats({})]}
                voterCounts={counted([
                    {channel: EMessageChannel.SMS, voters: 12040, not_offered: 0},
                ])}
            />
        )
        expect(
            Array.from(document.querySelectorAll("tr[data-channel]")).map((tr) =>
                tr.getAttribute("data-channel")
            )
        ).toEqual(["WHATSAPP", "SMS"])
        const cells = Array.from(row(EMessageChannel.SMS).querySelectorAll("td")).map(
            (cell) => cell.textContent
        )
        expect(cells).toEqual([
            "messaging.channel.SMS",
            "12,040",
            "6",
            "0",
            "4",
            "messaging.deliveryUnavailable",
            "2",
            "1",
            "3",
        ])
    })

    it("links Failed to the channel's failed messages in Logs", () => {
        const onShowFailed = jest.fn()
        render(
            <MessagingDeliveryStatus
                config={config}
                accounts={[account({}), whatsapp]}
                stats={[stats({})]}
                onShowFailed={onShowFailed}
            />
        )
        fireEvent.click(
            screen.getByRole("button", {name: /messagingEvent.delivery.showFailed.*SMS/})
        )
        expect(onShowFailed).toHaveBeenCalledWith(EMessageChannel.SMS)
    })

    it("says the counts are unavailable instead of showing 0", () => {
        render(
            <MessagingDeliveryStatus
                config={config}
                accounts={[account({}), whatsapp]}
                unavailable
                onShowFailed={jest.fn()}
            />
        )
        expect(screen.getByRole("status")).toHaveTextContent("messagingEvent.delivery.loadError")
        expect(screen.queryByRole("button", {name: /showFailed/})).toBeNull()
        expect(within(row(EMessageChannel.SMS)).queryByText("0")).toBeNull()
    })
})

describe("MessagingDeliveryStatus rows and voter counts (R8-01, R8-02)", () => {
    const config = configWith([[EMessageChannel.SMS, "sms-1", [EMessagePurpose.NOTICE]]])
    const counts = (
        channels: IVoterCounts["channels"],
        computedAt: string | null = "2026-10-09T08:00:00Z"
    ): IVoterCounts => ({
        total: channels.reduce((sum, entry) => sum + entry.voters, 0),
        channels,
        status: {
            computed_at: computedAt,
            pending: false,
            outcome: computedAt ? "COUNTED" : null,
        } as IVoterCounts["status"],
    })
    const failedOn = (channel: EMessageChannel | null): IChannelMessageStats => ({
        channel,
        sent_last_24h: 0,
        queued: 0,
        accepted: 0,
        delivered: 0,
        failed: 3,
        unknown: 0,
        unresolved: 0,
    })
    const rows = () =>
        Array.from(document.querySelectorAll("tr[data-channel]")).map((tr) =>
            tr.getAttribute("data-channel")
        )

    it("shows every channel voters or messages are on, those the event doesn't offer said so", () => {
        render(
            <MessagingDeliveryStatus
                config={config}
                accounts={[account({})]}
                stats={[failedOn(EMessageChannel.EMAIL), failedOn(null)]}
                voterCounts={counts([
                    {channel: EMessageChannel.WHATSAPP, voters: 2, not_offered: 2},
                    {channel: EMessageChannel.SMS, voters: 5, not_offered: 0},
                    {channel: EMessageChannel.EMAIL, voters: 40000, not_offered: 40000},
                ])}
            />
        )
        expect(rows()).toEqual(["WHATSAPP", "SMS", "EMAIL", "NONE"])
        expect(screen.getAllByText("messagingEvent.delivery.notConfigured")).toHaveLength(2)
        const email = within(row(EMessageChannel.EMAIL))
        expect(email.getByText("40,000")).toBeVisible()
        // Its refused messages are counted, not "unavailable".
        expect(email.queryByText("messaging.deliveryUnavailable")).toBeNull()
        expect(
            within(row(EMessageChannel.SMS)).queryByText("messagingEvent.delivery.notConfigured")
        ).toBeNull()
        expect(screen.getByText(/messaging.voterCount.countedAt/)).toBeVisible()
    })

    it("shows the voters and messages of an event without a configuration", () => {
        render(
            <MessagingDeliveryStatus
                config={configWith([])}
                accounts={[]}
                stats={[failedOn(EMessageChannel.EMAIL)]}
                voterCounts={counts([{channel: null, voters: 4, not_offered: 0}])}
            />
        )
        expect(rows()).toEqual(["EMAIL", "NONE"])
        expect(screen.queryByText("messagingEvent.delivery.empty")).toBeNull()
    })

    it("says no channel is in use only with no voter or message on any", () => {
        render(
            <MessagingDeliveryStatus
                config={configWith([])}
                accounts={[]}
                stats={[]}
                voterCounts={counts([])}
                onRecount={jest.fn()}
            />
        )
        expect(screen.getByText("messagingEvent.delivery.empty")).toBeVisible()
        expect(screen.getByRole("button", {name: "messaging.voterCount.recount"})).toBeEnabled()
    })

    it("shows no voter count before the first count, and asks for a recount", () => {
        const onRecount = jest.fn()
        render(
            <MessagingDeliveryStatus
                config={config}
                accounts={[account({})]}
                stats={[]}
                voterCounts={counts([], null)}
                onRecount={onRecount}
            />
        )
        expect(within(row(EMessageChannel.SMS)).getAllByRole("cell")[1]).toHaveTextContent("–")
        expect(screen.getByText("messaging.voterCount.never")).toBeVisible()
        fireEvent.click(screen.getByRole("button", {name: "messaging.voterCount.recount"}))
        expect(onRecount).toHaveBeenCalled()
    })

    it("can't ask again while a recount is on its way, and says a count took too long", () => {
        render(
            <MessagingDeliveryStatus
                config={config}
                accounts={[account({})]}
                stats={[]}
                voterCounts={{
                    ...counts([{channel: EMessageChannel.SMS, voters: 5, not_offered: 0}]),
                    status: {
                        computed_at: "2026-10-09T08:00:00Z",
                        pending: true,
                        outcome: "TIMED_OUT",
                    } as IVoterCounts["status"],
                }}
                onRecount={jest.fn()}
            />
        )
        expect(screen.getByRole("button", {name: "messaging.voterCount.recount"})).toBeDisabled()
        const status = screen.getByText(/messaging.voterCount.countedAt/)
        expect(status).toHaveTextContent("messaging.voterCount.timedOut")
        expect(status).toHaveTextContent("messaging.voterCount.previous")
        expect(status).toHaveTextContent("messaging.voterCount.pending")
    })
})

describe("delivery without a callback address (R4-04)", () => {
    it("shows WhatsApp delivery as unavailable, with the reason, never 0", () => {
        const noCallback = {
            ...whatsapp,
            status: {
                connected: true,
                production_access: true,
                callback_blockers: [EReadinessBlocker.CALLBACK_NOT_CONFIGURED],
                callback_reasons: ["Callback address not configured"],
            },
        }
        render(
            <MessagingDeliveryStatus
                config={configWith([[EMessageChannel.WHATSAPP, "wa-1", [EMessagePurpose.NOTICE]]])}
                accounts={[noCallback]}
                stats={[
                    {
                        channel: EMessageChannel.WHATSAPP,
                        queued: 0,
                        accepted: 100,
                        delivered: 0,
                        failed: 0,
                        unknown: 0,
                        unresolved: 0,
                    },
                ]}
            />
        )
        const cell = within(row(EMessageChannel.WHATSAPP)).getByText(
            "messaging.deliveryUnavailable"
        )
        expect(cell.closest("td")).toHaveAttribute(
            "title",
            "messaging.reason.CALLBACK_NOT_CONFIGURED"
        )
    })
})

describe("voters with no contact channel (R4-11)", () => {
    it("shows their failures on a row of their own, never under Email", () => {
        render(
            <MessagingDeliveryStatus
                config={configWith([[EMessageChannel.SMS, "sms-1", [EMessagePurpose.NOTICE]]])}
                accounts={[account({})]}
                stats={[
                    {
                        channel: EMessageChannel.SMS,
                        queued: 0,
                        accepted: 1,
                        delivered: 0,
                        failed: 2,
                        unknown: 0,
                        unresolved: 0,
                    },
                    {
                        channel: null,
                        queued: 0,
                        accepted: 0,
                        delivered: 0,
                        failed: 7,
                        unknown: 0,
                        unresolved: 0,
                    },
                ]}
            />
        )
        const noChannel = document.querySelector('tr[data-channel="NONE"]') as HTMLElement
        expect(noChannel).not.toBeNull()
        expect(within(noChannel).getByText("messaging.reason.NO_CONTACT_CHANNEL")).toBeVisible()
        expect(within(noChannel).getByText("7")).toBeVisible()
        expect(row(EMessageChannel.EMAIL)).toBeNull()
    })

    it("links their failures to Logs (Follow-up 2)", () => {
        const onShowFailed = jest.fn()
        render(
            <MessagingDeliveryStatus
                config={configWith([[EMessageChannel.SMS, "sms-1", [EMessagePurpose.NOTICE]]])}
                accounts={[account({})]}
                stats={[
                    {
                        channel: null,
                        queued: 0,
                        accepted: 0,
                        delivered: 0,
                        failed: 7,
                        unknown: 0,
                        unresolved: 0,
                    },
                ]}
                onShowFailed={onShowFailed}
            />
        )
        fireEvent.click(
            screen.getByRole("button", {name: /messagingEvent.delivery.showFailedNoChannel/})
        )
        expect(onShowFailed).toHaveBeenCalledWith(null)
    })
})

describe("a configuration copied from the old storage that didn't validate", () => {
    it("says it needs saving again, and why", () => {
        render(<MigratedConfigNotice updatedBy="migration: needs saving again" />)
        expect(screen.getByRole("alert")).toHaveTextContent("messagingEvent.needsSavingAgain")
        expect(screen.getByRole("alert")).toHaveTextContent("messagingEvent.needsSavingAgainHelp")
    })

    it("says nothing for a configuration saved through the action, or copied and valid", () => {
        const {container} = render(<MigratedConfigNotice updatedBy="migration" />)
        expect(container).toBeEmptyDOMElement()
        render(<MigratedConfigNotice updatedBy={null} />)
        expect(screen.queryByRole("alert")).toBeNull()
    })
})
