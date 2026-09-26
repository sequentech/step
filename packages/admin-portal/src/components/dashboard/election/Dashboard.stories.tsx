// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, electionRecord} from "@/__stories__/fixtures"
import {
    EStoryPermissions,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../../ui-essentials/.storybook/globals"
import {pending} from "../../../../../ui-essentials/.storybook/screens"
import DashboardElection from "./Dashboard"
import {
    ipAddressRows,
    statsDateRange,
    votersByChannel,
    votesPerDay,
} from "../__stories__/DashboardFixture"

interface Scenario {
    /** What the statistics query does. */
    stats: "records" | "loading" | "error"
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture() {
    const {permissions, workflow} = useStoryGlobals()
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} role={permissions}>
            <RecordContextProvider value={electionRecord(workflow)}>
                <DashboardElection />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const progressDefect = {
    expectedFailure: {
        reason: "The dashboard's loading spinner is a progressbar without an accessible name.",
        a11y: ["aria-progressbar-name"],
    },
}

const meta = {
    title: "Admin/Dashboard/Election/DashboardElection",
    component: DashboardElection,
    args: {stats: "records"},
    argTypes: {stats: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "React-admin row selection of the IP address list labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({ip_address: ipAddressRows()})
        graphql = graphqlBoundary(
            {
                GetElectionStats: () => {
                    if (args.stats === "loading") return pending()
                    if (args.stats === "error") throw new Error("Synthetic stats unavailable")
                    return {
                        data: {
                            stats: {
                                total_distinct_voters: 38,
                                total_areas: 2,
                                voters_by_channel: votersByChannel,
                                votes_per_day: votesPerDay,
                            },
                            users: {count: 1204},
                            election: [{statistics: {num_emails_sent: 12, num_sms_sent: 3}}],
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const statsCalls = () => graphql.calls.filter(({name}) => name === "GetElectionStats")

const card = (canvasElement: HTMLElement, label: string) =>
    within(canvasElement).getByText(label).parentElement as HTMLElement

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        const canvas = within(canvasElement)
        const {permissions} = readStoryGlobals(globals)
        const eligible = await canvas.findByText("Eligible Voters")
        await expect(within(eligible.parentElement as HTMLElement).getByText("1,204")).toBeVisible()
        await expect(within(card(canvasElement, "Actual Voters")).getByText("38")).toBeVisible()
        await expect(within(card(canvasElement, "SMS sent")).getByText("3")).toBeVisible()
        expect(statsCalls()).toEqual([
            {
                name: "GetElectionStats",
                variables: {
                    tenantId: TENANT_ID,
                    electionEventId: STORY_IDS.event,
                    electionId: STORY_IDS.election,
                    electionAlias: "Council",
                    ...statsDateRange(),
                    timeResolution: "day",
                    bucketCount: 7,
                },
                headers: {},
            },
        ])
        if (permissions === EStoryPermissions.ADMIN) {
            await expect(await canvas.findByRole("row", {name: /192\.0\.2\.10/})).toBeVisible()
            expect(data.calls[0]?.args[1]).toMatchObject({
                filter: {
                    tenant_id: TENANT_ID,
                    election_event_id: STORY_IDS.event,
                    election_id: STORY_IDS.election,
                },
            })
        } else {
            expect(canvas.queryByText("IP Addresses")).not.toBeInTheDocument()
        }
    },
}

export const Loading: Story = {
    args: {stats: "loading"},
    parameters: progressDefect,
    play: async ({canvasElement}) => {
        await waitFor(() => expect(statsCalls()).toHaveLength(1))
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
        expect(within(canvasElement).queryByText("Eligible Voters")).not.toBeInTheDocument()
    },
}

export const StatsUnavailable: Story = {
    args: {stats: "error"},
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const eligible = await canvas.findByText("Eligible Voters")
        await expect(within(eligible.parentElement as HTMLElement).getByText("-")).toBeVisible()
        // The votes chart shows it has no data instead of loading forever.
        expect(canvas.queryByRole("progressbar")).toBeNull()
        const votes = canvas.getByText("Votes over time").closest(".MuiPaper-root") as HTMLElement
        await expect(within(votes).getByText("-")).toBeVisible()
    },
}

export const WithoutIpAddressPermission: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByText("Eligible Voters")
        expect(within(canvasElement).queryByText("IP Addresses")).not.toBeInTheDocument()
        expect(data.calls).toEqual([])
    },
}

export const ChangeTheTimeRange: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("combobox", {name: "Time range"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "30d"}))
        await waitFor(() =>
            expect(statsCalls().at(-1)?.variables).toMatchObject({
                timeResolution: "day",
                bucketCount: 30,
            })
        )
    },
}
