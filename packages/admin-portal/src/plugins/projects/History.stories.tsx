// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {History} from "./History"
import {Permission} from "./model"
import {
    PROJECT_IDS,
    STORY_TIMEZONE,
    projectsBackend,
    storyHost,
} from "./__stories__/projectsFixture"

const backend = projectsBackend()

const meta = {
    title: "Admin/Plugins/History",
    component: History,
    args: {
        host: storyHost(backend, [Permission.Read]),
        projectId: PROJECT_IDS.plan,
        timezone: STORY_TIMEZONE,
        revision: 0,
    },
} satisfies Meta<typeof History>
export default meta
type Story = StoryObj<typeof meta>

/** The project's history includes its tasks' changes, newest first, in its timezone. */
export const ProjectHistory: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Changed Test voting")).toBeVisible()
        await expect(canvas.getByText("Imported 7 tasks")).toBeVisible()
        await expect(canvas.getByText("planner · 2026-10-17 11:10")).toBeVisible()
    },
}

/** A task's history, which stays readable after the task is deleted. */
export const TaskHistory: Story = {
    args: {taskId: backend.tasks()[2].id},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Changed")).toBeVisible()
        expect(canvas.queryByText("Imported 7 tasks")).toBeNull()
    },
}
