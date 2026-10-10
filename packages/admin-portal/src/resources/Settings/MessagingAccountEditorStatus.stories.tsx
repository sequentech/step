// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {
    EMessagingProvider,
    EProviderApproval,
    IAccountCheckDetails,
    IMessagingAccount,
} from "@/types/messaging"
import {AccountStatusDetails} from "./MessagingAccountEditorStatus"
import {messengerAccount, viberAccount, whatsappAccount} from "./__stories__/MessagingFixture"

enum EStatusStory {
    WHATSAPP = "WHATSAPP",
    WHATSAPP_UNCHECKED = "WHATSAPP_UNCHECKED",
    WHATSAPP_NOT_READ = "WHATSAPP_NOT_READ",
    MESSENGER = "MESSENGER",
    VIBER_REFUSED = "VIBER_REFUSED",
}

interface Scenario {
    account: EStatusStory
}

const checked = (
    account: IMessagingAccount,
    details: IAccountCheckDetails,
    overrides: Partial<NonNullable<IMessagingAccount["status"]>> = {}
): IMessagingAccount => ({
    ...account,
    status: {
        connected: true,
        production_access: true,
        approved_templates: {OTP: ["en"], NOTICE: ["en"]},
        checked_at: "2026-10-09T08:00:00Z",
        reason: null,
        details,
        ...overrides,
    },
})

const WHATSAPP_DETAILS: IAccountCheckDetails = {
    token: {valid: true, expires_at: "2026-12-01T00:00:00Z", missing_scopes: []},
    whatsapp_number: {
        status: "CONNECTED",
        name_status: "APPROVED",
        quality_rating: "GREEN",
        messaging_limit: "TIER_1K",
    },
    templates: [
        {name: "voting_code", language: "en", category: "AUTHENTICATION", status: "APPROVED"},
        {name: "election_notice", language: "en", category: "UTILITY", status: "APPROVED"},
        {name: "election_notice", language: "es", category: "UTILITY", status: "PENDING"},
    ],
    provider_approval: EProviderApproval.CONFIRMED,
    number_changes: [
        {
            masked_old_number: "+34*******101",
            masked_new_number: "+34*******202",
            voter_ids: ["voter-0001"],
            reported_at: "2026-10-08T17:30:00Z",
        },
    ],
}

const ACCOUNTS: Record<EStatusStory, () => IMessagingAccount> = {
    [EStatusStory.WHATSAPP]: () => checked(whatsappAccount(), WHATSAPP_DETAILS),
    // Its last check was of the number it had before.
    [EStatusStory.WHATSAPP_UNCHECKED]: () =>
        checked(whatsappAccount(), {
            ...WHATSAPP_DETAILS,
            identity: {
                provider: EMessagingProvider.WHATSAPP_CLOUD_API,
                ids: {business_account_id: "100000000000001", phone_number_id: "earlier"},
            },
        }),
    [EStatusStory.WHATSAPP_NOT_READ]: () =>
        checked(
            whatsappAccount(),
            {
                token: {
                    valid: true,
                    expires_at: null,
                    missing_scopes: ["whatsapp_business_management"],
                },
                templates: [],
                templates_reason: "The provider's templates couldn't be read",
            },
            {production_access: false, reason: "The number's status couldn't be read"}
        ),
    [EStatusStory.MESSENGER]: () =>
        checked(messengerAccount(), {
            token: {valid: true, expires_at: null, missing_scopes: []},
            messenger: {pages_messaging: true, subscribed: true, get_started: false},
            templates: [],
        }),
    [EStatusStory.VIBER_REFUSED]: () =>
        checked(
            viberAccount(),
            {partner_login: {accepted: false, reason: "Infobip refused the account's API key"}},
            {connected: false, reason: "Infobip refused the account's API key"}
        ),
}

const meta = {
    title: "Admin/Settings/AccountStatusDetails",
    component: AccountStatusDetails,
    args: {account: EStatusStory.WHATSAPP},
    argTypes: {account: {control: "inline-radio", options: Object.values(EStatusStory)}},
    parameters: {widgets: ["Row"]},
    render: (args) => (
        <AdminStoryProvider
            boundary={graphqlBoundary({})}
            roles={[IPermissions.MESSAGING_ACCOUNT_READ]}
        >
            <AccountStatusDetails account={ACCOUNTS[args.account]()} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const WhatsApp: Story = {
    parameters: {widgets: ["Row", "NumberChanges"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Provider status")).toBeVisible()
        await expect(canvas.getByText("High")).toBeVisible()
        await expect(canvas.getByText("TIER_1K")).toBeVisible()
        await expect(canvas.getByText(/Reported by the provider's check/)).toBeVisible()
        await expect(canvas.getByText("voting_code")).toBeVisible()
        await expect(canvas.getByText("In review")).toBeVisible()
        await expect(canvas.getByText("Voters who changed their WhatsApp number")).toBeVisible()
        await expect(canvas.getByText(/Voters: voter-0001/)).toBeVisible()
    },
}

export const IdentityChangedNotChecked: Story = {
    args: {account: EStatusStory.WHATSAPP_UNCHECKED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(/Until the account is checked again, codes and notices/)
        ).toBeVisible()
        expect(canvas.queryByText("Quality rating")).toBeNull()
    },
}

export const WhatTheCheckCouldNotRead: Story = {
    args: {account: EStatusStory.WHATSAPP_NOT_READ},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(/Missing permissions: whatsapp_business_management/)
        ).toBeVisible()
        await expect(canvas.getByText("The provider's templates couldn't be read")).toBeVisible()
    },
}

export const MessengerPage: Story = {
    args: {account: EStatusStory.MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/Without the button/)).toBeVisible()
        await expect(canvas.getByText("Valid · doesn't expire")).toBeVisible()
    },
}

export const ViberPartnerLoginRefused: Story = {
    args: {account: EStatusStory.VIBER_REFUSED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Refused")).toBeVisible()
        await expect(
            canvas.getByText(/Viber templates are entered by an administrator/)
        ).toBeVisible()
    },
}
