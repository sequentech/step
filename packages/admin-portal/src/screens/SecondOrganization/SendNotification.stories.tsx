// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Send Notification of a second organization: "Each voter's channel" names
// its own accounts, for its own notice channels.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    OTHER_CLIENT,
    SECOND_ORG_ACCOUNT_NAMES,
    secondOrgAccounts,
    secondOrgAudiencePreview,
    secondOrgEvent,
    secondOrgEventConfig,
    secondOrgTemplates,
} from "@/__stories__/SecondOrganizationFixture"
import {AudienceSelection, SendTemplate} from "@/resources/User/SendTemplate"
import {IPermissions} from "@/types/keycloak"
import {EMessageChannel} from "@/types/messaging"

interface Scenario {
    close: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Screens/Admin/Second organization/Send notification",
    component: SendTemplate,
    args: {close: fn()},
    parameters: {
        router: {initialEntries: ["/sequent_backend_election_event/voters"]},
        expectedFailure: {
            reason: "The audience, method and alias selects have no accessible name, and the three section regions share one.",
            a11y: ["aria-input-field-name", "landmark-unique"],
        },
    },
    beforeEach: async () => {
        // The messaging operations are not in the generated schema yet.
        graphql = graphqlBoundary(
            {
                GetMessagingAccountSummaries: () => ({
                    data: {sequent_backend_messaging_account: secondOrgAccounts()},
                }),
                GetEventMessagingConfig: () => ({
                    data: {
                        sequent_backend_messaging_event_config: [
                            {
                                election_event_id: EVENT_ID,
                                config: secondOrgEventConfig(),
                                version: 1,
                            },
                        ],
                    },
                }),
                // Only its own event's whole audience has rows.
                GetMessagingAudiencePreview: ({variables}) => {
                    const preview = secondOrgAudiencePreview()
                    const own =
                        variables.electionEventId === EVENT_ID &&
                        variables.audienceSelection === AudienceSelection.ALL_USERS
                    return {
                        data: {
                            messaging_audience_preview: own
                                ? preview
                                : {...preview, total: 0, will_send: 0, channels: []},
                        },
                    }
                },
                CreateScheduledEvent: () => ({data: {createScheduledEvent: {id: storyId(10, 8)}}}),
            },
            {schema: false}
        )
        data = resourceBoundary({
            sequent_backend_template: secondOrgTemplates(),
            sequent_backend_election_event: [secondOrgEvent()],
            sequent_backend_election: [],
        })
        await graphql.ready
    },
    render: ({close}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={[IPermissions.MESSAGING_CONFIG_READ]}
        >
            <SendTemplate
                ids={[]}
                audienceSelection={AudienceSelection.ALL_USERS}
                electionEventId={EVENT_ID}
                close={close}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** Each voter's channel: its notice channels, each sent from its own account. */
export const EachVotersChannel: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const table = within(await canvas.findByRole("table", {name: "Each voter's channel"}))
        await expect(await canvas.findByText("Voters in this audience: 1,855")).toBeVisible()
        // The pension board's WhatsApp voters aren't sent to.
        await expect(canvas.getByText("Will get the notification: 1,830")).toBeVisible()
        expect(
            table
                .getAllByRole("row")
                .slice(1)
                .map((row) => row.getAttribute("data-channel"))
        ).toEqual([EMessageChannel.WHATSAPP, EMessageChannel.VIBER, EMessageChannel.EMAIL])
        for (const name of [
            SECOND_ORG_ACCOUNT_NAMES.WHATSAPP,
            SECOND_ORG_ACCOUNT_NAMES.VIBER,
            SECOND_ORG_ACCOUNT_NAMES.EMAIL,
        ]) {
            await expect(await table.findByRole("cell", {name})).toBeVisible()
        }
        // The pension board offers only Email: its WhatsApp voters get nothing on WhatsApp.
        await expect(
            canvas.getByText("25 of them don't get it: Channel not offered for notices.")
        ).toBeVisible()
        // The rows are its own event's audience: the screen asked for them.
        const previews = graphql.calls.filter(({name}) => name === "GetMessagingAudiencePreview")
        expect(previews.length).toBeGreaterThan(0)
        for (const call of previews) {
            expect(call).toMatchObject({
                variables: {electionEventId: EVENT_ID, audienceSelection: "ALL_USERS"},
                headers: {"x-hasura-role": IPermissions.MESSAGING_CONFIG_READ},
            })
        }
        expect(canvasElement.textContent).not.toMatch(OTHER_CLIENT)
    },
}
