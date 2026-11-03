// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {StoryObj} from "@storybook/react-vite"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {expect, fn, userEvent, within} from "storybook/test"
import {ProjectView} from "./ProjectView"
import {Operation, Permission} from "./model"
import {
    PROJECT_IDS,
    SAMPLE_PROJECT_NAME,
    projectsBackend,
    storyHost,
} from "./__stories__/projectsFixture"

let backend: ReturnType<typeof projectsBackend>

interface Scenario {
    permissions: Permission[]
    onBack: () => void
}

const meta = {
    title: "Admin/Plugins/ProjectView",
    component: ProjectView,
    args: {
        permissions: [Permission.Read, Permission.Write, Permission.Manage],
        onBack: fn(),
    },
    beforeEach: () => {
        backend = projectsBackend()
    },
    render: ({permissions, onBack}) => (
        <ProjectView
            host={storyHost(backend, permissions)}
            projectId={PROJECT_IDS.plan}
            onBack={onBack}
        />
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** Selecting a task opens its editor below the full-width timeline; moving it keeps the edit. */
export const EditBelowTheTimeline: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(SAMPLE_PROJECT_NAME)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: /^Test voting, /}))
        const title = await canvas.findByRole("textbox", {name: "Title"})
        await userEvent.type(title, " (mock polls)")
        canvas.getByRole("button", {name: /^Test voting, /}).focus()
        await userEvent.keyboard("{ArrowRight}")
        await expect(title).toHaveValue("Test voting (mock polls)")
        await expect(canvas.getByLabelText(/^Start/)).toHaveValue("2028-04-04T08:00")
        expect(backend.operations).not.toContain(Operation.SaveTask)
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await expect(await canvas.findByText(/Saved “Test voting \(mock polls\)”/)).toBeVisible()
    },
}

/** Leaving with unsaved changes asks first; keeping them stays on the task. */
export const UnsavedChanges: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: /^Voting, /}))
        await userEvent.type(await canvas.findByRole("textbox", {name: "Title"}), "!")
        await userEvent.click(canvas.getByRole("button", {name: "Projects"}))
        await expect(await canvas.findByText(/You have unsaved changes/)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Keep editing"}))
        expect(args.onBack).not.toHaveBeenCalled()
        await expect(canvas.getByRole("textbox", {name: "Title"})).toHaveValue("Voting!")
    },
}

/** With projects-read only: nothing can be added, moved or saved. */
export const Viewer: Story = {
    args: {permissions: [Permission.Read]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(/View only/)).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Add task"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: /^Enrollment, /}))
        await expect(await canvas.findByText("Task details")).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
    },
}
