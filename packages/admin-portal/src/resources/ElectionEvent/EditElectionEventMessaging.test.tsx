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
    EReadinessPolicy,
    IChannelMessageStats,
    IEventMessagingConfig,
    IMessagingAccount,
} from "@/types/messaging"
import {emptyEventMessagingConfig} from "@/services/messaging"
import {
    MessagingChannels,
    MessagingDeliveryStatus,
    MessagingTemplateBindings,
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
    const group = () =>
        within(screen.getByRole("group", {name: /messagingEvent.templates.row.*"position":1/}))

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
        voters: 40,
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
                        voters: 9,
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

    it("shows voters, the last 24 hours and Unresolved apart, in channel order", () => {
        render(
            <MessagingDeliveryStatus
                config={config}
                accounts={[account({}), whatsapp]}
                stats={[stats({})]}
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
            "40",
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
