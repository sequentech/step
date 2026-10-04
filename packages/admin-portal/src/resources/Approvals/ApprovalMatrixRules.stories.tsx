// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import type {Mock} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ApprovalMatrixRules} from "./ApprovalMatrixRules"
import {humanizeField} from "./approvalMatrix"
import {
    MatrixScreen,
    associationMatrix,
    comelecMatrix,
    setUpMatrix,
} from "./__stories__/ApprovalMatrixFixture"

interface Scenario {
    matrix: "comelec" | "association"
    canEdit: boolean
    onEdit: Mock<(index: number | null) => void>
    onMove: Mock<(index: number, offset: -1 | 1) => void>
    onDelete: Mock<(index: number) => void>
    onAdd: Mock<() => void>
}

const meta = {
    title: "Admin/Approvals/ApprovalMatrixRules",
    component: ApprovalMatrixRules,
    args: {
        matrix: "comelec",
        canEdit: true,
        onEdit: fn(),
        onMove: fn(),
        onDelete: fn(),
        onAdd: fn(),
    },
    argTypes: {matrix: {control: "inline-radio", options: ["comelec", "association"]}},
    parameters: {
        expectedFailure: {
            reason: "The status chips put white text on light colours.",
            a11y: ["color-contrast"],
        },
    },
    beforeEach: () => setUpMatrix(),
    render: ({matrix, canEdit, ...actions}) => (
        <MatrixScreen canEdit={canEdit}>
            <ApprovalMatrixRules
                matrix={matrix === "comelec" ? comelecMatrix() : associationMatrix()}
                canEdit={canEdit}
                fieldLabel={humanizeField}
                {...actions}
            />
        </MatrixScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const table = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("table", {name: "Rules"})

export const Editable: Story = {
    play: async ({canvasElement, args}) => {
        const rules = within(await table(canvasElement))
        await expect(
            rules.getByRole("row", {name: /1 Voter already enrolled At most 1 field differs/})
        ).toBeVisible()
        await expect(rules.getByRole("button", {name: "Move rule 1 up"})).toBeDisabled()
        await expect(rules.getByRole("button", {name: "Move rule 7 down"})).toBeDisabled()

        await userEvent.click(rules.getByRole("button", {name: "Edit rule 2"}))
        expect(args.onEdit).toHaveBeenLastCalledWith(1)
        await userEvent.click(rules.getByRole("button", {name: "Edit the last rule"}))
        expect(args.onEdit).toHaveBeenLastCalledWith(null)
        await userEvent.click(rules.getByRole("button", {name: "Move rule 3 up"}))
        expect(args.onMove).toHaveBeenLastCalledWith(2, -1)
        await userEvent.click(rules.getByRole("button", {name: "Move rule 3 down"}))
        expect(args.onMove).toHaveBeenLastCalledWith(2, 1)
        await userEvent.click(rules.getByRole("button", {name: "Delete rule 7"}))
        expect(args.onDelete).toHaveBeenLastCalledWith(6)
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Add Rule"}))
        expect(args.onAdd).toHaveBeenCalledTimes(1)
    },
}

export const ReadOnly: Story = {
    args: {canEdit: false},
    play: async ({canvasElement}) => {
        const rules = within(await table(canvasElement))
        await expect(rules.getAllByRole("columnheader")).toHaveLength(4)
        expect(within(canvasElement).queryByRole("button")).toBeNull()
    },
}

export const Association: Story = {
    args: {matrix: "association"},
    play: async ({canvasElement}) => {
        const rules = within(await table(canvasElement))
        await expect(
            rules.getByRole("row", {name: /3 All compared fields match ACCEPTED -/})
        ).toBeVisible()
        await expect(
            rules.getByRole("row", {name: /Otherwise PENDING No Matching Voter/})
        ).toBeVisible()
    },
}
