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
import type {IMonitoringEditorApi} from "./editor/api"
import {EMonitoringConfigKind} from "./editor/types"
import {THEME_YAML, WIDGET_YAML, fakeEditorApi} from "./editor/storyFixtures"

interface Scenario {
    role: EStoryPermissions
    list: EMonitoringListScenario
    electionId?: string
    lock: EMonitoringLock
    onEditDashboard: (dashboardId: string) => void
    /** Given, the tab wires in the editor over it instead of recording `onEditDashboard`. */
    editorApi?: IMonitoringEditorApi
}

let graphql: ReturnType<typeof graphqlBoundary>

const LEGACY = "Standard election event dashboard"

/** Found, then visible once the dialog or notice has finished coming in. */
const shown = async (found: Promise<HTMLElement>) => {
    const element = await found
    await waitFor(() => expect(element).toBeVisible())
}

/** The fixture's `overview` dashboard as the editor reads it. */
const OVERVIEW_YAML = [
    "id: overview",
    "title: Monitoring overview",
    "layout:",
    ...[
        "turnout-summary",
        "turnout-by-group",
        "voting-activity",
        "poll-status",
        "attack-detections",
    ].map((widget, index) => `  - widget: ${widget}\n    width: ${index ? 6 : 12}`),
    "",
].join("\n")

/** The editor over the fixture's event: its dashboard, and each widget as a copy of one. */
const tabEditorApi = () =>
    fakeEditorApi({
        getConfig: fn(async ({kind, key}) => ({
            kind,
            key,
            yaml:
                kind === EMonitoringConfigKind.DASHBOARD
                    ? OVERVIEW_YAML
                    : kind === EMonitoringConfigKind.THEME
                      ? THEME_YAML
                      : WIDGET_YAML.replace("id: turnout-by-group", `id: ${key}`),
            revision: 7,
        })),
    })

function Fixture({role, electionId, lock, onEditDashboard, editorApi}: Scenario) {
    return (
        <AdminStoryProvider boundary={graphql} role={role}>
            <MonitoringDashboardTab
                electionEventId={STORY_IDS.event}
                electionId={electionId}
                lock={lock}
                legacy={<p>{LEGACY}</p>}
                actions={editorApi ? undefined : {onEditDashboard}}
                editorApi={editorApi}
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
        await expect(await canvas.findByRole("combobox", {name: "Section"})).toHaveTextContent(
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
        await canvas.findByRole("combobox", {name: "Section"})
        expect(canvas.queryByRole("button", {name: "Edit dashboard"})).toBeNull()
    },
}

export const LockedDownEvent: Story = {
    args: {lock: EMonitoringLock.LOCKED_DOWN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("combobox", {name: "Section"})
        expect(canvas.queryByRole("button", {name: "Edit dashboard"})).toBeNull()
    },
}

export const LockNotKnownYet: Story = {
    args: {lock: EMonitoringLock.UNKNOWN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("combobox", {name: "Section"})
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

/** Edit dashboard, with the editor wired in, opens the dashboard editor on the dashboard shown. */
export const EditorEditsTheDashboardShown: Story = {
    args: {editorApi: tabEditorApi()},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const page = within(canvasElement.ownerDocument.body)
        await userEvent.click(await canvas.findByRole("button", {name: "Edit dashboard"}))
        await shown(page.findByRole("dialog", {name: "Editing dashboard · Monitoring overview"}))
        expect(args.editorApi?.getConfig).toHaveBeenCalledWith(
            expect.objectContaining({kind: EMonitoringConfigKind.DASHBOARD, key: "overview"})
        )
    },
}

/** Configure widget, from a widget's menu, previews the widget on the dashboard shown. */
export const EditorConfiguresAWidget: Story = {
    args: {editorApi: tabEditorApi()},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const page = within(canvasElement.ownerDocument.body)
        await userEvent.click(
            await canvas.findByRole("button", {name: "Actions for Turnout by group"})
        )
        await userEvent.click(await page.findByRole("menuitem", {name: "Configure widget"}))
        await shown(page.findByRole("dialog", {name: /Configure widget/}))
        await waitFor(() =>
            expect(args.editorApi?.renderWidget).toHaveBeenCalledWith(
                expect.objectContaining({dashboard_id: "overview", widget_id: "turnout-by-group"})
            )
        )
    },
}

/** Duplicate, from a widget's menu, saves a copy, places it after the widget and reloads the view. */
export const EditorDuplicatesAWidget: Story = {
    args: {editorApi: tabEditorApi()},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const page = within(canvasElement.ownerDocument.body)
        await userEvent.click(
            await canvas.findByRole("button", {name: "Actions for Turnout by group"})
        )
        const loads = () =>
            graphql.calls.filter(({name}) => name === "MonitoringGetDashboard").length
        const before = loads()
        await userEvent.click(await page.findByRole("menuitem", {name: "Duplicate"}))
        await shown(
            page.findByText("Added turnout-by-group-copy, a copy of the widget, to the dashboard.")
        )
        expect(args.editorApi?.saveConfig).toHaveBeenCalledWith(
            expect.objectContaining({
                kind: EMonitoringConfigKind.WIDGET,
                key: "turnout-by-group-copy",
            })
        )
        expect(args.editorApi?.saveConfig).toHaveBeenCalledWith(
            expect.objectContaining({
                kind: EMonitoringConfigKind.DASHBOARD,
                key: "overview",
                expected_revision: 7,
                yaml: expect.stringContaining(
                    "  - widget: turnout-by-group\n    width: 6\n  - widget: turnout-by-group-copy\n"
                ),
            })
        )
        await waitFor(() => expect(loads()).toBeGreaterThan(before))
    },
}
