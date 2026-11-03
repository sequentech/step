// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {TaskEditor} from "./TaskEditor"
import {Task} from "./model"
import {STORY_TIMEZONE, projectsBackend} from "./__stories__/projectsFixture"

const testVoting = projectsBackend().tasks()[2].data

/** Holds the draft, as the project screen does. */
const Editor = (args: React.ComponentProps<typeof TaskEditor>) => {
    const [draft, setDraft] = useState<Task>(args.draft)
    return <TaskEditor {...args} draft={draft} setDraft={setDraft} />
}

const meta = {
    title: "Admin/Plugins/TaskEditor",
    component: TaskEditor,
    args: {
        draft: testVoting,
        setDraft: fn(),
        timezone: STORY_TIMEZONE,
        editable: true,
        busy: false,
        existing: true,
        onSave: fn(),
        onDelete: fn(),
        onCancel: fn(),
        onPendingChange: fn(),
    },
    render: (args) => <Editor {...args} />,
} satisfies Meta<typeof TaskEditor>
export default meta
type Story = StoryObj<typeof meta>

/** Dates are entered in the project's timezone; an end before the start blocks saving. */
export const EndBeforeStart: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Times are in Asia/Manila.")).toBeVisible()
        const end = canvas.getByLabelText(/^End/)
        await userEvent.clear(end)
        await userEvent.type(end, "2028-04-02T17:00")
        await expect(await canvas.findByText("The end must be after the start.")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(args.onSave).not.toHaveBeenCalled()
    },
}

/** A time a clock change repeats is not guessed: the user chooses the occurrence. */
export const RepeatedTime: Story = {
    args: {
        timezone: "Europe/Madrid",
        draft: {
            ...testVoting,
            start: "2026-10-24T09:00:00+02:00",
            end: "2026-10-26T09:00:00+01:00",
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const start = canvas.getByLabelText(/^Start/)
        await userEvent.clear(start)
        await userEvent.type(start, "2026-10-25T02:30")
        await expect(await canvas.findByText(/happens twice in Europe\/Madrid/)).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        await userEvent.click(canvas.getByRole("radio", {name: "At UTC+01:00"}))
        await expect(start).toHaveAccessibleDescription("UTC+01:00")
        await expect(canvas.getByRole("button", {name: "Save"})).toBeEnabled()
    },
}

/** With projects-read only, the same fields are read-only and there is no Save. */
export const ReadOnly: Story = {
    args: {editable: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("textbox", {name: "Title"})).toHaveAttribute("readonly")
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
    },
}
