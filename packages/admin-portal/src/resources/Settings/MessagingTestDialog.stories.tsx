// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EMessageAttemptState} from "@/types/messaging"
import {MessagingTestDialog} from "./MessagingTestDialog"
import {
    VIBER_ACCOUNT_ID,
    WHATSAPP_ACCOUNT_ID,
    viberAccount,
    whatsappAccount,
} from "./__stories__/MessagingFixture"

enum EStoryOutcome {
    ACCEPTED = "ACCEPTED",
    DELIVERED = "DELIVERED",
    UNKNOWN = "UNKNOWN",
    FAILED = "FAILED",
    REQUEST_FAILS = "REQUEST_FAILS",
}

interface Scenario {
    outcome: EStoryOutcome
    /** Which account is tested. */
    whatsapp: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>

const STATES: Record<EStoryOutcome, EMessageAttemptState> = {
    [EStoryOutcome.ACCEPTED]: EMessageAttemptState.ACCEPTED,
    [EStoryOutcome.DELIVERED]: EMessageAttemptState.DELIVERED,
    [EStoryOutcome.UNKNOWN]: EMessageAttemptState.UNKNOWN,
    [EStoryOutcome.FAILED]: EMessageAttemptState.FAILED,
    [EStoryOutcome.REQUEST_FAILS]: EMessageAttemptState.FAILED,
}

const meta = {
    title: "Admin/Settings/MessagingTestDialog",
    component: MessagingTestDialog,
    args: {outcome: EStoryOutcome.ACCEPTED, whatsapp: false},
    argTypes: {outcome: {control: "inline-radio", options: Object.values(EStoryOutcome)}},
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary({
            TestMessagingAccount: () => {
                if (args.outcome === EStoryOutcome.REQUEST_FAILS) {
                    throw new Error("Synthetic messaging service unavailable")
                }
                return {
                    data: {
                        test_messaging_account: {
                            message_id: "message-1",
                            state: STATES[args.outcome],
                            reason:
                                args.outcome === EStoryOutcome.FAILED
                                    ? "Template not approved for this language"
                                    : null,
                        },
                    },
                }
            },
        })
        await graphql.ready
    },
    render: ({whatsapp}) => (
        <AdminStoryProvider boundary={graphql}>
            <MessagingTestDialog
                account={whatsapp ? whatsappAccount() : viberAccount()}
                languages={["en", "es"]}
                onClose={fn()}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const sendTest = async () => {
    const dialog = within(await within(document.body).findByRole("dialog"))
    await userEvent.type(
        dialog.getByRole("textbox", {name: "Phone number (E.164)"}),
        "+34600000001"
    )
    await userEvent.click(dialog.getByRole("button", {name: "Send test message"}))
    return dialog
}

export const Accepted: Story = {
    play: async () => {
        const dialog = await sendTest()
        await expect(await dialog.findByText("Accepted")).toBeVisible()
        await expect(dialog.getByText(/This does not mean the voter received it/)).toBeVisible()
        expect(graphql.calls[0]).toMatchObject({
            name: "TestMessagingAccount",
            variables: {id: VIBER_ACCOUNT_ID, purpose: "OTP", language: "en"},
        })
    },
}

export const Delivered: Story = {
    args: {outcome: EStoryOutcome.DELIVERED},
    play: async () => {
        const dialog = await sendTest()
        await expect(await dialog.findByText("Delivered")).toBeVisible()
    },
}

export const Unknown: Story = {
    args: {outcome: EStoryOutcome.UNKNOWN},
    play: async () => {
        const dialog = await sendTest()
        await expect(await dialog.findByText("Unknown")).toBeVisible()
        await expect(dialog.getByText(/Delivery is not confirmed yet/)).toBeVisible()
    },
}

export const Failed: Story = {
    args: {outcome: EStoryOutcome.FAILED},
    play: async () => {
        const dialog = await sendTest()
        await expect(await dialog.findByText("Failed")).toBeVisible()
        await expect(dialog.getByText(/Template not approved for this language/)).toBeVisible()
    },
}

export const RequestFails: Story = {
    args: {outcome: EStoryOutcome.REQUEST_FAILS},
    play: async () => {
        const dialog = await sendTest()
        await expect(await dialog.findByText("The test message could not be sent.")).toBeVisible()
    },
}

export const WhatsAppNeedsTheApprovedTemplate: Story = {
    args: {whatsapp: true},
    play: async () => {
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.type(
            dialog.getByRole("textbox", {name: "Phone number (E.164)"}),
            "+34600000001"
        )
        const send = dialog.getByRole("button", {name: "Send test message"})
        await expect(send).toBeDisabled()
        await userEvent.type(dialog.getByRole("textbox", {name: "Approved template"}), "otp_en")
        await expect(dialog.getByText(/enter the provider's language code/)).toBeVisible()
        const language = dialog.getByRole("combobox", {name: "Language"})
        await userEvent.clear(language)
        await userEvent.type(language, "en_US")
        await userEvent.click(send)
        await expect(await dialog.findByText("Accepted")).toBeVisible()
        expect(graphql.calls[0]).toMatchObject({
            name: "TestMessagingAccount",
            variables: {
                id: WHATSAPP_ACCOUNT_ID,
                purpose: "OTP",
                language: "en_US",
                template: "otp_en",
            },
        })
    },
}
