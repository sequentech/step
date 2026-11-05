// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {StoryObj} from "@storybook/react-vite"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {expect, userEvent, within} from "storybook/test"
import Projects from "./Projects"
import {Operation, Permission} from "./model"
import {SAMPLE_PROJECT_NAME, projectsBackend, storyHost} from "./__stories__/projectsFixture"

let backend: ReturnType<typeof projectsBackend>

interface Scenario {
    permissions: Permission[]
}

const meta = {
    title: "Admin/Plugins/Projects",
    component: Projects,
    args: {permissions: [Permission.Read, Permission.Write, Permission.Manage]},
    beforeEach: () => {
        backend = projectsBackend()
    },
    render: ({permissions}) => <Projects host={storyHost(backend, permissions)} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** The list of the tenant's projects; opening one shows its timeline. */
export const ListAndOpen: Story = {
    parameters: {widgets: ["ProjectList"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const plan = await canvas.findByRole("button", {name: SAMPLE_PROJECT_NAME})
        await expect(canvas.getByRole("tab", {name: "Active (2)"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect(canvas.getByRole("button", {name: "New project"})).toBeVisible()
        await userEvent.click(plan)
        await expect(await canvas.findByRole("button", {name: /^Enrollment, /})).toBeVisible()
        expect(backend.operations).toEqual([
            Operation.ListProjects,
            Operation.GetProject,
            Operation.History,
        ])
    },
}

/** A viewer can open projects but not create or import them. */
export const Viewer: Story = {
    args: {permissions: [Permission.Read]},
    parameters: {widgets: ["ProjectList"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("button", {name: SAMPLE_PROJECT_NAME})
        expect(canvas.queryByRole("button", {name: "New project"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Import"})).toBeNull()
    },
}
