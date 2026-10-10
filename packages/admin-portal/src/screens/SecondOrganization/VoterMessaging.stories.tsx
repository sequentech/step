// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The voter drawer's messaging summary for voters of a second organization:
// a WhatsApp voter with a Spanish number, and one connected to its Page.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    OTHER_CLIENT,
    SECOND_ORG_MASKED_NUMBER,
    SECOND_ORG_SENDERS,
    SECOND_ORG_VOTER_EMAIL,
    secondOrgAccounts,
    secondOrgFailedNotice,
    secondOrgMessengerVoter,
    secondOrgWhatsAppVoter,
} from "@/__stories__/SecondOrganizationFixture"
import {VoterMessagingSummary} from "@/resources/User/VoterMessagingSummary"
import {IPermissions} from "@/types/keycloak"
import type {ILedgerMessage} from "@/types/messaging"

interface Scenario {
    attributes: Record<string, string[]>
    failure: ILedgerMessage | null
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Screens/Admin/Second organization/Voter messaging",
    component: VoterMessagingSummary,
    args: {attributes: secondOrgWhatsAppVoter(), failure: secondOrgFailedNotice()},
    argTypes: {attributes: {control: "object"}},
    beforeEach: async ({args}) => {
        // The messaging operations are not in the generated schema yet.
        graphql = graphqlBoundary(
            {
                GetVoterUnsettledMessages: () => ({
                    data: {sequent_backend_message: args.failure ? [args.failure] : []},
                }),
                GetVoterMessageAttempts: () => ({
                    data: {sequent_backend_message: args.failure ? [args.failure] : []},
                }),
                GetMessagingAccountSummaries: () => ({
                    data: {sequent_backend_messaging_account: secondOrgAccounts()},
                }),
            },
            {schema: false}
        )
        await graphql.ready
    },
    render: ({attributes}) => (
        <AdminStoryProvider boundary={graphql} roles={[IPermissions.MESSAGING_CONFIG_READ]}>
            <VoterMessagingSummary
                attributes={attributes}
                email={SECOND_ORG_VOTER_EMAIL}
                userId={STORY_IDS.user}
                electionEventId={EVENT_ID}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** A WhatsApp voter: masked Spanish number, and the last notice refused at their Post. */
export const WhatsAppVoter: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("WhatsApp")).toBeVisible()
        await expect(canvas.getByText(SECOND_ORG_MASKED_NUMBER)).toBeVisible()
        expect(canvas.queryByText(/600111214/)).toBeNull()
        await expect(
            await canvas.findByText(/^Failed · .* · Channel not offered for notices$/)
        ).toBeVisible()
        expect(canvasElement.textContent).not.toMatch(OTHER_CLIENT)
    },
}

/** A voter connected to the organization's Messenger Page: the Page is named. */
export const MessengerVoter: Story = {
    args: {attributes: secondOrgMessengerVoter(), failure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(SECOND_ORG_SENDERS.MESSENGER)).toBeVisible()
        expect(canvasElement.textContent).not.toMatch(OTHER_CLIENT)
    },
}
