// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// VOTE-FREEZE (D6): a failed seal is an incident the election event's
// Dashboard and Publish tab show, with each failed ballot box and its reason.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, electionRecord} from "@/__stories__/fixtures"
import {FailedSealsBanner} from "./FailedSealsBanner"

interface Scenario {
    /** Whether a seal of the event failed. */
    failed: boolean
    /** The viewer's roles. */
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Dashboard/FailedSealsBanner",
    component: FailedSealsBanner,
    args: {failed: true, roles: ["election-dashboard-tab"]},
    beforeEach: async ({args}) => {
        data = resourceBoundary({sequent_backend_election: [electionRecord()]})
        graphql = graphqlBoundary(
            {
                GetFailedBallotBoxSeals: () => ({
                    data: {
                        sequent_backend_ballot_box_seal: args.failed
                            ? [
                                  {
                                      id: "00000000-0000-4000-8000-000000000001",
                                      election_id: STORY_IDS.election,
                                      area_id: STORY_IDS.area,
                                      area: {id: STORY_IDS.area, name: "Andorra"},
                                      area_name: "Andorra",
                                      election_name: "Council",
                                      failure_reason:
                                          "a ballot does not match its Ballot ID (stored 3a91…, content hashes to 77c0…)",
                                  },
                              ]
                            : [],
                    },
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles}>
            <FailedSealsBanner
                electionEventId={EVENT_ID}
                presentation={{ballot_box_seal_policy: "seal-at-close"}}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const AFailedSeal: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("1 ballot box could not be sealed")).toBeVisible()
        await expect(
            await canvas.findByText("Council, Andorra: A ballot does not match its Ballot ID.")
        ).toBeVisible()
        expect(graphql.calls[0]?.headers).toMatchObject({
            "x-hasura-role": "election-dashboard-tab",
        })
    },
}

export const NoIncident: Story = {
    args: {failed: false},
    play: async ({canvasElement}) => {
        await new Promise((resolve) => setTimeout(resolve, 300))
        expect(within(canvasElement).queryByRole("alert")).toBeNull()
    },
}

export const WithoutARoleThatReadsSeals: Story = {
    args: {roles: ["election-event-read"]},
    play: async ({canvasElement}) => {
        await new Promise((resolve) => setTimeout(resolve, 300))
        expect(graphql.calls).toEqual([])
        expect(within(canvasElement).queryByRole("alert")).toBeNull()
    },
}
