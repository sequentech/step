// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {Navigate, Route, Routes} from "react-router-dom"
import {initCore} from "@sequentech/ui-core"
import {tenantRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import {
    TreeStory,
    treeServices,
    type TreeServices,
} from "@/components/menu/items/__stories__/ElectionTreeFixture"
import {CustomLayout} from "@/components/CustomLayout"
import {groupRoles} from "@/__stories__/storyAuth"
import {EStoryPermissions} from "../../../ui-essentials/.storybook/globals"
import {PluginProvider} from "./PluginProvider"
import {PluginRoute} from "./PluginRoutes"
import {PluginState} from "./contracts"
import {SAMPLE_PROJECT_NAME, projectsServices} from "./projects/__stories__/projectsFixture"
import {Permission} from "./projects/model"

interface Scenario {
    /** Projects permissions besides the administrator's other roles. */
    projects: Permission[]
    state: PluginState
}

let services: TreeServices
let projects: ReturnType<typeof projectsServices>

const otherRoles = () =>
    groupRoles(EStoryPermissions.ADMIN).filter((role) => !role.startsWith("projects-"))

/** The portal's shell with the Projects plugin's route, as the application renders it. */
function Fixture({projects: permissions}: Scenario) {
    return (
        <TreeStory
            services={services}
            role={EStoryPermissions.ADMIN}
            roles={[...otherRoles(), ...permissions]}
            tenant={tenantRecord as Sequent_Backend_Tenant}
            createProvider={false}
        >
            <PluginProvider>
                <CustomLayout>
                    <Routes>
                        <Route path="/plugins/:id/*" element={<PluginRoute />} />
                        <Route path="/settings" element={<h1>Settings</h1>} />
                        <Route path="*" element={<Navigate to="/plugins/projects" replace />} />
                    </Routes>
                </CustomLayout>
            </PluginProvider>
        </TreeStory>
    )
}

// The shell's own defects, as the CustomLayout stories record them.
const shellDefects = {
    reason: "The header's banner is nested in the app bar's, and the side menu holds non-items in a menu.",
    a11y: [
        "aria-required-children",
        "landmark-banner-is-top-level",
        "landmark-no-duplicate-banner",
        "landmark-unique",
    ],
}

const meta = {
    title: "Admin/Plugins/PluginRoute",
    component: PluginRoute,
    args: {
        projects: [Permission.Read, Permission.Write, Permission.Manage],
        state: PluginState.Enabled,
    },
    parameters: {expectedFailure: shellDefects},
    beforeEach: async ({args}) => {
        await initCore()
        projects = projectsServices({state: args.state})
        services = treeServices({handlers: projects.handlers})
        await services.graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const openPlan = async (canvasElement: HTMLElement) => {
    const canvas = within(canvasElement)
    await userEvent.click(await canvas.findByRole("button", {name: SAMPLE_PROJECT_NAME}))
    return canvas.findByRole("button", {name: /^Enrollment, /})
}

/** The list, with the plugin's menu item after Templates. */
export const ProjectList: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("button", {name: SAMPLE_PROJECT_NAME})).toBeVisible()
        // At phone width the side menu is a closed drawer.
        if (window.matchMedia("(min-width: 600px)").matches) {
            await expect(canvas.getByRole("menuitem", {name: "Projects"})).toBeVisible()
        }
        expect(canvas.getByRole("button", {name: "New project"})).toBeVisible()
        expect(projects.operations).toEqual(["list-projects"])
    },
}

/** The timeline fills the content width; no task table beside it. */
export const Timeline: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const enrollment = await openPlan(canvasElement)
        await expect(enrollment).toBeVisible()
        expect(
            within(canvas.getByRole("list", {name: "7 tasks"})).getAllByRole("listitem")
        ).toHaveLength(7)
        expect(canvas.queryByRole("table")).toBeNull()
        const timeline = canvas.getByRole("region", {name: "Timeline"})
        const content = timeline.parentElement as HTMLElement
        await waitFor(() =>
            expect(timeline.getBoundingClientRect().width).toBeGreaterThan(
                content.getBoundingClientRect().width - 2
            )
        )
        expect(canvas.getByText(/All dates and times are in Asia\/Manila/)).toBeVisible()
    },
}

/** Selecting a task opens its details and editor below the timeline. */
export const TaskDetailsBelow: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await openPlan(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: /^Test voting, /}))
        const panel = await canvas.findByRole("region", {name: "Edit task"})
        const timeline = canvas.getByRole("region", {name: "Timeline"})
        expect(panel.getBoundingClientRect().top).toBeGreaterThan(
            timeline.getBoundingClientRect().bottom
        )
        await expect(within(panel).getByRole("textbox", {name: "Title"})).toHaveValue("Test voting")
    },
}

/** Pointing at a task shows a short preview; focus shows the same. */
export const HoverPreview: Story = {
    play: async ({canvasElement}) => {
        const enrollment = await openPlan(canvasElement)
        await userEvent.hover(enrollment)
        const preview = await within(document.body).findByTestId("projects-task-preview")
        await expect(within(preview).getByText("Enrollment")).toBeVisible()
        await expect(within(preview).getByText(/2028-03-31 17:00/)).toBeVisible()
    },
}

/** With projects-read only: details are read-only and nothing can be changed. */
export const Viewer: Story = {
    args: {projects: [Permission.Read]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await openPlan(canvasElement)
        await expect(canvas.getByText(/View only/)).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Add task"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: /^Enrollment, /}))
        const panel = await canvas.findByRole("region", {name: "Task details"})
        expect(within(panel).queryByRole("button", {name: "Save"})).toBeNull()
    },
}

/** Leaving through the menu with unsaved changes asks first. */
export const UnsavedChangesGuard: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await openPlan(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: /^Test voting, /}))
        const title = await canvas.findByRole("textbox", {name: "Title"})
        await userEvent.type(title, " (mock polls)")
        await userEvent.click(canvas.getByRole("menuitem", {name: "Settings"}))
        await expect(await canvas.findByText(/You have unsaved changes/)).toBeVisible()
        expect(canvas.queryByRole("heading", {name: "Settings"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Keep editing"}))
        await expect(title).toHaveValue("Test voting (mock polls)")
    },
}

/** Turned off for the tenant: no menu item, and the route explains why. */
export const TurnedOff: Story = {
    args: {state: PluginState.Disabled},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(/Projects is not turned on/)).toBeVisible()
        expect(canvas.queryByRole("menuitem", {name: "Projects"})).toBeNull()
        expect(projects.operations).toEqual([])
    },
}
