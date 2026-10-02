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
    IEventMessagingConfig,
    IMessagingAccount,
} from "@/types/messaging"
import {emptyEventMessagingConfig} from "@/services/messaging"
import {MessagingChannels, MessagingDeliveryStatus} from "./EditElectionEventMessaging"

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
