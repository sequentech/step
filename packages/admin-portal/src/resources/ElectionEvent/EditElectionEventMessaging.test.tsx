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
        out_of_window: EOutOfWindowPolicy.DISABLED,
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
        expect(next.notice_fallback).toEqual([EMessageChannel.SMS])
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

describe("the out-of-window policy", () => {
    const renderChannels = (config: IEventMessagingConfig, canEdit = true) => {
        const onChange = jest.fn()
        render(
            <MessagingChannels
                config={config}
                accounts={[account({}), messenger, whatsapp]}
                errors={[]}
                canEdit={canEdit}
                onChange={onChange}
            />
        )
        return onChange
    }

    it("is a choice on a channel with a window and free-text notices, and nowhere else", () => {
        renderChannels(
            configWith([
                [EMessageChannel.SMS, "sms-1", []],
                [EMessageChannel.WHATSAPP, "wa-1", []],
                [EMessageChannel.MESSENGER, "page-1", [EMessagePurpose.NOTICE]],
            ])
        )
        expect(
            within(row(EMessageChannel.MESSENGER)).getByText("messagingEvent.outOfWindow.help")
        ).toBeVisible()
        expect(
            within(row(EMessageChannel.MESSENGER)).getByRole("combobox", {
                name: /messagingEvent.outOfWindow.label/,
            })
        ).not.toHaveAttribute("aria-disabled")
        expect(
            within(row(EMessageChannel.SMS)).queryByText("messagingEvent.outOfWindow.help")
        ).toBeNull()
        expect(
            within(row(EMessageChannel.WHATSAPP)).queryByText("messagingEvent.outOfWindow.help")
        ).toBeNull()
    })

    it("asks for a notice template once utility messages are chosen", () => {
        const utility = configWith([
            [EMessageChannel.MESSENGER, "page-1", [EMessagePurpose.NOTICE]],
        ])
        utility.channels[0].out_of_window = EOutOfWindowPolicy.UTILITY_MESSAGES
        renderChannels(utility)
        expect(screen.getByRole("status")).toHaveTextContent(
            "messagingEvent.outOfWindow.noTemplate"
        )
    })

    it("stops asking when a notice template is bound", () => {
        const utility = configWith([
            [EMessageChannel.MESSENGER, "page-1", [EMessagePurpose.NOTICE]],
        ])
        utility.channels[0].out_of_window = EOutOfWindowPolicy.UTILITY_MESSAGES
        utility.channels[0].templates = [
            {purpose: EMessagePurpose.NOTICE, language: "en", provider_template: "vote_reminder"},
        ]
        renderChannels(utility)
        expect(screen.queryByRole("status")).toBeNull()
    })

    it("is read only without the permission", () => {
        renderChannels(
            configWith([[EMessageChannel.MESSENGER, "page-1", [EMessagePurpose.NOTICE]]]),
            false
        )
        expect(
            screen.getByRole("combobox", {name: /messagingEvent.outOfWindow.label/})
        ).toHaveAttribute("aria-disabled", "true")
    })
})

describe("readiness confirmed by an administrator", () => {
    it("lets a purpose be switched on without a check", () => {
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
        expect(purposeSwitch(EMessageChannel.WHATSAPP, EMessagePurpose.OTP)).toBeEnabled()
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

    it("shows the administrator's confirmation rather than a check result", () => {
        renderBindings(bound(), {
            accounts: [{...whatsapp, readiness: EReadinessPolicy.ADMIN_CONFIRMED}],
        })
        expect(group().getByText("messagingEvent.templates.approval.ADMIN_CONFIRMED")).toBeVisible()
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
    it("shows delivery as unavailable for a provider without receipts, never 0", () => {
        render(
            <MessagingDeliveryStatus
                config={configWith([
                    [EMessageChannel.SMS, "sms-1", [EMessagePurpose.NOTICE]],
                    [EMessageChannel.WHATSAPP, "wa-1", [EMessagePurpose.NOTICE]],
                ])}
                accounts={[account({}), whatsapp]}
                stats={{
                    SMS_ACCEPTED: {aggregate: {count: 4}},
                    WHATSAPP_DELIVERED: {aggregate: {count: 0}},
                    WHATSAPP_ACCEPTED: {aggregate: {count: 2}},
                }}
            />
        )
        const sms = within(row(EMessageChannel.SMS))
        expect(sms.getByText("messaging.deliveryUnavailable")).toBeVisible()
        expect(sms.getByText("4")).toBeVisible()
        const whatsappRow = within(row(EMessageChannel.WHATSAPP))
        expect(whatsappRow.queryByText("messaging.deliveryUnavailable")).toBeNull()
        expect(whatsappRow.getByText("2")).toBeVisible()
    })
})
