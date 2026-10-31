// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {Timeline} from "./Timeline"
import {STORY_TIMEZONE, projectsBackend} from "./__stories__/projectsFixture"

const tasks = projectsBackend().tasks()

const meta = {
    title: "Admin/Plugins/Timeline",
    component: Timeline,
    args: {
        tasks,
        timezone: STORY_TIMEZONE,
        selected: null,
        editable: true,
        pending: [],
        onSelect: fn(),
        onMove: fn(),
    },
} satisfies Meta<typeof Timeline>
export default meta
type Story = StoryObj<typeof meta>

/** Keyboard focus shows the same short preview as pointing; arrows move by 24 hours. */
export const KeyboardPreviewAndMove: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(within(canvas.getByRole("list")).getAllByRole("listitem")).toHaveLength(7)
        const ert = canvas.getByRole("button", {name: /^Election Readiness Test \(ERT\), /})
        ert.focus()
        await userEvent.keyboard("{ArrowRight}")
        expect(args.onMove).toHaveBeenCalledWith(
            expect.objectContaining({id: tasks[1].id}),
            1,
            false
        )
        await userEvent.keyboard("{Shift>}{ArrowLeft}{/Shift}")
        expect(args.onMove).toHaveBeenLastCalledWith(
            expect.objectContaining({id: tasks[1].id}),
            -1,
            true
        )
        await userEvent.keyboard("{ArrowDown}")
        await expect(canvas.getByRole("button", {name: /^Test voting, /})).toHaveFocus()
        await userEvent.keyboard("{Enter}")
        expect(args.onSelect).toHaveBeenCalledWith(expect.objectContaining({id: tasks[2].id}))
    },
}

/** Pointing at a task shows its status, duration and dates in the project's timezone. */
export const HoverPreview: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.hover(canvas.getByRole("button", {name: /^Voting, /}))
        const preview = await within(document.body).findByTestId("projects-task-preview")
        await expect(within(preview).getByText("12 h")).toBeVisible()
        await expect(within(preview).getByText(/2028-05-08 07:00/)).toBeVisible()
    },
}

/** Read-only: tasks can be selected and previewed, never moved. */
export const ReadOnly: Story = {
    args: {editable: false, selected: tasks[3].id},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const ftl = canvas.getByRole("button", {name: /^Final Testing and Lockdown \(FTL\), /})
        await expect(ftl).toHaveAttribute("aria-current", "true")
        ftl.focus()
        await userEvent.keyboard("{ArrowRight}")
        expect(args.onMove).not.toHaveBeenCalled()
        await expect(canvas.getByText(/Select it to see its details below/)).toBeVisible()
    },
}
