// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {ImportProject, NewProjectMode} from "./ImportProject"
import {Operation, Permission} from "./model"
import {projectsBackend, storyHost} from "./__stories__/projectsFixture"

let backend: ReturnType<typeof projectsBackend>

const meta = {
    title: "Admin/Plugins/ImportProject",
    component: ImportProject,
    args: {
        mode: NewProjectMode.Import,
        host: storyHost(projectsBackend(), [Permission.Read, Permission.Write]),
        onClose: fn(),
        onCreated: fn(),
    },
    beforeEach: () => {
        backend = projectsBackend()
    },
    render: (args) => (
        <ImportProject {...args} host={storyHost(backend, [Permission.Read, Permission.Write])} />
    ),
} satisfies Meta<typeof ImportProject>
export default meta
type Story = StoryObj<typeof meta>

/** Every row is checked before anything is saved; one invalid row blocks the import. */
export const PreviewWithErrors: Story = {
    play: async ({args}) => {
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.type(dialog.getByRole("textbox", {name: "Project name"}), "Copy")
        const file = new File(["id,title,type,start,end,status,notes\n"], "plan.csv", {
            type: "text/csv",
        })
        const input = document.body.querySelector<HTMLInputElement>("input[type=file]")
        await userEvent.upload(input as HTMLInputElement, file)
        await expect(
            await dialog.findByText("Use an ISO 8601 date and time with a UTC offset")
        ).toBeVisible()
        await expect(dialog.getByText(/1 of 2 rows has errors/)).toBeVisible()
        await expect(dialog.getByRole("button", {name: "Create project"})).toBeDisabled()
        expect(backend.operations).toEqual([Operation.PreviewImport])
        expect(args.onCreated).not.toHaveBeenCalled()
    },
}

/** Creating an empty project asks only for its name and IANA timezone. */
export const CreateProject: Story = {
    args: {mode: NewProjectMode.Create},
    play: async () => {
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(dialog.getByRole("button", {name: "Create project"})).toBeDisabled()
        await userEvent.type(dialog.getByRole("textbox", {name: "Project name"}), "Logistics")
        await expect(dialog.getByRole("button", {name: "Create project"})).toBeEnabled()
        expect(document.body.querySelector("input[type=file]")).toBeNull()
    },
}
