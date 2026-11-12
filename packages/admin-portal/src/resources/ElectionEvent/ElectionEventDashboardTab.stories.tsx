// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createRef} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {initCore} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    STORY_SETTINGS,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, eventRecord} from "@/__stories__/fixtures"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"
import {
    EMonitoringListScenario,
    monitoringHandlers,
} from "@/components/monitoring/__stories__/MonitoringFixture"
import {ElectionEventDashboardTab} from "./ElectionEventDashboardTab"
// The tab loads both dashboards lazily, and a cold dev server can take longer to
// serve one than a story may wait.
import "@/components/dashboard/election-event/Dashboard"
import "@/components/monitoring/MonitoringDashboardTab"

interface Scenario {
    /** Whether the event shows configured dashboards or the standard one. */
    monitoring: EMonitoringListScenario
    /** Whether the event offers kiosk voting. */
    kiosk: boolean
    role: EStoryPermissions
    handleChildMount: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
const refreshRef = createRef<HTMLButtonElement>()

function Fixture({kiosk, role, handleChildMount}: Scenario) {
    const event = eventRecord(undefined, {
        voting_channels: {online: true, kiosk, early_voting: false, telephone: false},
    })
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} role={role}>
            <RecordContextProvider value={event}>
                <ElectionEventDashboardTab
                    refreshRef={refreshRef}
                    handleChildMount={handleChildMount}
                />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/ElectionEventDashboardTab",
    component: ElectionEventDashboardTab,
    args: {
        monitoring: EMonitoringListScenario.CONFIGURED,
        kiosk: false,
        role: EStoryPermissions.ADMIN,
        handleChildMount: fn(),
    },
    beforeEach: async ({args}) => {
        // The voter links come from sequent-core.
        await initCore()
        data = resourceBoundary({sequent_backend_tally_session: [], ip_address: []})
        graphql = graphqlBoundary(
            {
                ...monitoringHandlers({list: args.monitoring}),
                GetElectionEventStats: () => ({
                    data: {
                        stats: {
                            total_eligible_voters: 1204,
                            total_distinct_voters: 38,
                            total_areas: 2,
                            total_elections: 1,
                            voters_by_channel: [],
                            votes_per_day: [],
                        },
                        election_event: [{statistics: {num_emails_sent: 12, num_sms_sent: 3}}],
                    },
                }),
                ListKeysCeremony: () => ({
                    data: {list_keys_ceremony: {items: [], total: {aggregate: {count: 0}}}},
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const eventUrl = (portal: string | undefined, action: string) =>
    `${portal}/tenant/${TENANT_ID}/event/${STORY_IDS.event}/${action}`

const voterLinks = (canvasElement: HTMLElement) =>
    within(canvasElement)
        .getAllByRole("link")
        .filter((link) => link.textContent?.startsWith("Voter "))
        .map((link) => [link.textContent, link.getAttribute("href")])

const ONLINE_LINKS = [
    ["Voter Login URL", eventUrl(STORY_SETTINGS.VOTING_PORTAL_URL, "login")],
    ["Voter Enroll URL", eventUrl(STORY_SETTINGS.VOTING_PORTAL_URL, "enroll")],
]

export const ConfiguredDashboards: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("combobox", {name: "Dashboard"})).toBeVisible()
        expect(canvas.queryByText("Eligible Voters")).toBeNull()
        await waitFor(() => expect(voterLinks(canvasElement)).toEqual(ONLINE_LINKS))
    },
}

export const StandardDashboard: Story = {
    args: {monitoring: EMonitoringListScenario.LEGACY},
    play: async ({canvasElement, args}) => {
        await expect(await within(canvasElement).findByText("Eligible Voters")).toBeVisible()
        // Once, under the standard dashboard.
        await waitFor(() => expect(voterLinks(canvasElement)).toEqual(ONLINE_LINKS))
        expect(args.handleChildMount).toHaveBeenCalled()
    },
}

export const KioskEvent: Story = {
    args: {kiosk: true},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("combobox", {name: "Dashboard"})
        ).toBeVisible()
        await waitFor(() =>
            expect(voterLinks(canvasElement)).toEqual([
                ...ONLINE_LINKS,
                [
                    "Voter Enroll Kiosk URL",
                    `${eventUrl(STORY_SETTINGS.KIOSK_VOTING_PORTAL_URL, "login")}?kiosk`,
                ],
            ])
        )
    },
}
