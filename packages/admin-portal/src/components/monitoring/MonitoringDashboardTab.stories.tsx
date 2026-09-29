// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"
import {EMonitoringLock} from "./useMonitoringPermissions"
import {MonitoringDashboardTab} from "./MonitoringDashboardTab"
import {EMonitoringListScenario, POSTS, monitoringHandlers} from "./__stories__/MonitoringFixture"

interface Scenario {
    role: EStoryPermissions
    list: EMonitoringListScenario
    electionId?: string
    lock: EMonitoringLock
    onEditDashboard: (dashboardId: string) => void
}

let graphql: ReturnType<typeof graphqlBoundary>

const LEGACY = "Standard election event dashboard"

function Fixture({role, electionId, lock, onEditDashboard}: Scenario) {
    return (
        <AdminStoryProvider boundary={graphql} role={role}>
            <MonitoringDashboardTab
                electionEventId={STORY_IDS.event}
                electionId={electionId}
                lock={lock}
                legacy={<p>{LEGACY}</p>}
                actions={{onEditDashboard}}
            />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Monitoring/MonitoringDashboardTab",
    component: MonitoringDashboardTab,
    args: {
        role: EStoryPermissions.ADMIN,
        list: EMonitoringListScenario.CONFIGURED,
        lock: EMonitoringLock.OPEN,
        onEditDashboard: fn(),
    },
    beforeEach: async ({args}) => {
        window.sessionStorage.clear()
        graphql = graphqlBoundary(
            monitoringHandlers({
                list: args.list,
                pinnedPost: args.electionId ? POSTS.madrid : null,
            }),
            {schema: true}
        )
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Configured: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("combobox", {name: "Dashboard"})).toHaveTextContent(
            "Monitoring overview"
        )
        expect(canvas.queryByText(LEGACY)).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Edit dashboard"}))
        await expect(args.onEditDashboard).toHaveBeenCalledWith("overview")
    },
}

export const LegacyMode: Story = {
    args: {list: EMonitoringListScenario.LEGACY},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText(LEGACY)).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toEqual(["MonitoringListDashboards"])
    },
}

export const MonitoringUnavailable: Story = {
    args: {list: EMonitoringListScenario.ERROR},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "Monitoring dashboards could not be loaded, so the standard dashboard is shown."
            )
        ).toBeVisible()
        await expect(canvas.getByText(LEGACY)).toBeVisible()
    },
}

export const Loading: Story = {
    args: {list: EMonitoringListScenario.LOADING},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("progressbar", {name: "Loading monitoring"})
        ).toBeVisible()
    },
}

export const WithoutMonitoringPermission: Story = {
    args: {role: EStoryPermissions.ADMIN_LOCKDOWN},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText(LEGACY)).toBeVisible()
        // No monitoring request is made for a viewer without monitoring-view.
        expect(graphql.calls).toEqual([])
    },
}

export const ViewerWithoutConfigure: Story = {
    args: {role: EStoryPermissions.ADMIN_LIGHT},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("combobox", {name: "Dashboard"})
        expect(canvas.queryByRole("button", {name: "Edit dashboard"})).toBeNull()
    },
}

export const LockedDownEvent: Story = {
    args: {lock: EMonitoringLock.LOCKED_DOWN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("combobox", {name: "Dashboard"})
        expect(canvas.queryByRole("button", {name: "Edit dashboard"})).toBeNull()
    },
}

export const LockNotKnownYet: Story = {
    args: {lock: EMonitoringLock.UNKNOWN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("combobox", {name: "Dashboard"})
        // Until the event says whether it is locked down, nothing offers a change.
        expect(canvas.queryByRole("button", {name: "Edit dashboard"})).toBeNull()
    },
}

export const ElectionPage: Story = {
    args: {electionId: POSTS.madrid},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("combobox", {name: "Country"})
        // The election's Post is fixed, and with it its Region: neither is offered.
        expect(canvas.queryByRole("combobox", {name: "Post"})).toBeNull()
        expect(canvas.queryByRole("combobox", {name: "Region"})).toBeNull()
        await waitFor(() =>
            expect(
                graphql.calls.find(({name}) => name === "MonitoringRenderWidget")?.variables
            ).toEqual(expect.objectContaining({electionId: POSTS.madrid}))
        )
        expect(graphql.calls[0].variables).toEqual(
            expect.objectContaining({electionId: POSTS.madrid})
        )
    },
}
