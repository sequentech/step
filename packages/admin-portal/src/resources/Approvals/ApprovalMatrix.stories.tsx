// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Mock} from "storybook/test"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ApprovalMatrix} from "./ApprovalMatrix"
import {
    MatrixScreen,
    comelecMatrix,
    matrixCalls,
    setUpMatrix,
    unexpectedCalls,
    type MatrixServices,
} from "./__stories__/ApprovalMatrixFixture"

interface Scenario extends MatrixServices {
    /** The width of the screen, for a phone. */
    width?: number
    goBack: Mock<() => void>
}

const meta = {
    title: "Admin/Approvals/ApprovalMatrix",
    component: ApprovalMatrix,
    args: {saved: "built-in", canEdit: true, reads: "matrix", saves: "saved", goBack: fn()},
    argTypes: {
        saved: {control: "inline-radio", options: ["built-in", "association"]},
        reads: {control: "inline-radio", options: ["matrix", "loading", "error"]},
        saves: {control: "inline-radio", options: ["saved", "error"]},
    },
    parameters: {
        expectedFailure: {
            reason: "The status chips put white text on light colours.",
            a11y: ["color-contrast"],
        },
    },
    beforeEach: ({args}) => setUpMatrix(args),
    render: ({canEdit, goBack, width}) => (
        <MatrixScreen canEdit={canEdit}>
            <div data-testid="screen" style={{width}}>
                <ApprovalMatrix electionEventId={EVENT_ID} goBack={goBack} />
            </div>
        </MatrixScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const rules = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("table", {name: "Rules"})

const saveButton = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("button", {name: "Save"})

const dialog = async () => {
    const element = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

export const BuiltInVersion: Story = {
    play: async ({canvasElement}) => {
        const table = within(await rules(canvasElement))
        await expect(table.getAllByRole("row")).toHaveLength(9)
        await expect(
            table.getByRole("row", {name: /5 Exactly 1 field differs Embassy matches PENDING/})
        ).toBeVisible()
        await expect(
            table.getByRole("row", {name: /Otherwise REJECTED No Matching Voter/})
        ).toBeVisible()
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/Version 1 · Built-in rules/)).toBeVisible()
        await expect(saveButton(canvasElement)).toBeDisabled()
        await expect(await canvas.findByText("Rule 3 applies:")).toBeVisible()
        expect(unexpectedCalls()).toEqual([])
    },
}

export const AssociationVersion: Story = {
    args: {saved: "association"},
    play: async ({canvasElement}) => {
        const table = within(await rules(canvasElement))
        await expect(table.getAllByRole("row")).toHaveLength(5)
        await expect(
            table.getByRole("row", {name: /Otherwise PENDING No Matching Voter/})
        ).toBeVisible()
        await expect(within(canvasElement).getByText(/Version 3 · Saved .* by admin/)).toBeVisible()
        expect(within(canvasElement).queryByRole("combobox", {name: "Embassy"})).toBeNull()
    },
}

export const ReadOnly: Story = {
    args: {canEdit: false},
    play: async ({canvasElement}) => {
        const table = within(await rules(canvasElement))
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText("You can view and test the matrix, but not change it.")
        ).toBeVisible()
        await expect(
            canvas.getByText("First Name, Middle Name, Last Name, Date of birth, Embassy")
        ).toBeVisible()
        expect(table.queryByRole("button")).toBeNull()
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Add Rule"})).toBeNull()
        await expect(await canvas.findByText("Rule 3 applies:")).toBeVisible()
    },
}

export const EditARuleAndSave: Story = {
    play: async ({canvasElement}) => {
        const table = within(await rules(canvasElement))
        const canvas = within(canvasElement)
        await userEvent.click(table.getByRole("button", {name: "Edit rule 4"}))
        const editor = await dialog()
        await expect(editor.getByRole("heading", {name: "Edit Rule 4"})).toBeVisible()
        await userEvent.click(editor.getByRole("combobox", {name: "Decision"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "Send to manual review"})
        )
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        await expect(await editor.findByText("Choose the reason shown to the voter.")).toBeVisible()
        await userEvent.click(editor.getByRole("combobox", {name: "Reason Shown To The Voter"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "No Matching Voter"})
        )
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))

        await expect(
            await table.findByRole("row", {
                name: /4 Exactly 1 field differs Embassy differs PENDING/,
            })
        ).toBeVisible()
        await expect(canvas.getByText("Unsaved changes")).toBeVisible()
        await waitFor(() => expect(saveButton(canvasElement)).toBeEnabled())
        await userEvent.click(saveButton(canvasElement))
        const confirmation = await dialog()
        await expect(confirmation.getByText(/Save these rules as version 2\?/)).toBeVisible()
        await userEvent.click(confirmation.getByRole("button", {name: "Save"}))

        await expect(await canvas.findByText(/Version 2 · Saved .* by admin/)).toBeVisible()
        const expected = comelecMatrix()
        expected.rules[3].then = {...expected.rules[4].then}
        expect(matrixCalls("SaveApprovalMatrix")[0].variables).toEqual({
            electionEventId: EVENT_ID,
            matrix: expected,
        })
        await waitFor(() => expect(saveButton(canvasElement)).toBeDisabled())
    },
}

export const ReorderAddAndDelete: Story = {
    args: {saved: "association"},
    play: async ({canvasElement}) => {
        const table = within(await rules(canvasElement))
        await userEvent.click(table.getByRole("button", {name: "Move rule 1 down"}))
        await expect(
            await table.findByRole("row", {name: /^1 Identity entered manually/})
        ).toBeVisible()
        await userEvent.click(table.getByRole("button", {name: "Delete rule 3"}))
        await waitFor(() => expect(table.getAllByRole("row")).toHaveLength(4))

        await userEvent.click(within(canvasElement).getByRole("button", {name: "Add Rule"}))
        const editor = await dialog()
        await expect(editor.getByRole("heading", {name: "New Rule"})).toBeVisible()
        await userEvent.click(editor.getByRole("combobox", {name: "Identity Verification"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "Entered manually"})
        )
        await userEvent.click(editor.getByRole("combobox", {name: "Decision"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "Approve automatically"})
        )
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        await expect(
            await editor.findByText(
                "Enrollments whose identity was entered manually can't be approved automatically."
            )
        ).toBeVisible()
        await userEvent.click(editor.getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(table.getAllByRole("row")).toHaveLength(4))
        await expect(saveButton(canvasElement)).toBeEnabled()
    },
}

export const ChangeTheComparedFields: Story = {
    play: async ({canvasElement}) => {
        const table = within(await rules(canvasElement))
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("combobox", {name: "Embassy"})).toBeVisible()
        await userEvent.click(
            within(canvas.getByRole("button", {name: "Embassy"})).getByTestId("CancelIcon")
        )
        await expect(
            await table.findByRole("row", {name: /^4 Exactly 1 field differs ACCEPTED/})
        ).toBeVisible()
        await waitFor(() => expect(canvas.queryByRole("combobox", {name: "Embassy"})).toBeNull())
        await waitFor(() =>
            expect(matrixCalls("EvaluateApprovalMatrix").at(-1)?.variables).toMatchObject({
                enrollment: {
                    fields: {
                        firstName: "MATCHES",
                        middleName: "MATCHES",
                        lastName: "MATCHES",
                        dateOfBirth: "MATCHES",
                    },
                },
                matrix: {compared_fields: ["firstName", "middleName", "lastName", "dateOfBirth"]},
            })
        )
    },
}

export const SaveFails: Story = {
    args: {saves: "error"},
    play: async ({canvasElement}) => {
        const table = within(await rules(canvasElement))
        await userEvent.click(table.getByRole("button", {name: "Delete rule 7"}))
        await waitFor(() => expect(saveButton(canvasElement)).toBeEnabled())
        await userEvent.click(saveButton(canvasElement))
        const confirmation = await dialog()
        await userEvent.click(confirmation.getByRole("button", {name: "Save"}))
        await expect(
            await within(document.body).findByText("The approval matrix could not be saved")
        ).toBeVisible()
        await expect(within(canvasElement).getByText("Unsaved changes")).toBeVisible()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("progressbar", {name: "Approval Matrix"})
        ).toBeVisible()
    },
}

export const LoadFails: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText("The approval matrix could not be loaded.")
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Back"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const NarrowScreen: Story = {
    args: {width: 320},
    play: async ({canvasElement}) => {
        await expect(await rules(canvasElement)).toBeVisible()
        const screen = within(canvasElement).getByTestId("screen")
        expect(screen.scrollWidth).toBeLessThanOrEqual(screen.clientWidth)
        await expect(
            await within(canvasElement).findByRole("combobox", {name: "Embassy"})
        ).toBeVisible()
    },
}
