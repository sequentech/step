// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import type {Mock} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ApprovalMatrixRules, type RulePointer} from "./ApprovalMatrixRules"
import {EMatrixError, humanizeField} from "./approvalMatrix"
import {
    MatrixScreen,
    associationMatrix,
    comelecMatrix,
    setUpMatrix,
} from "./__stories__/ApprovalMatrixFixture"

interface Scenario {
    matrix: "comelec" | "association"
    canEdit: boolean
    /** The rule that decides the example of the test panel. */
    appliesTo?: RulePointer
    /** The rule that decided the enrollment the administrator came from. */
    cameFrom?: RulePointer
    problems?: Array<{code: EMatrixError; rule: number | null}>
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
    beforeEach: () => setUpMatrix(),
    render: ({matrix, canEdit, ...rest}) => (
        <MatrixScreen canEdit={canEdit}>
            <ApprovalMatrixRules
                matrix={matrix === "comelec" ? comelecMatrix() : associationMatrix()}
                canEdit={canEdit}
                fieldLabel={humanizeField}
                {...rest}
            />
        </MatrixScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** The rule cards in order; the last one is the rule that applies when no other does. */
const ruleCards = async (canvasElement: HTMLElement) =>
    within(await within(canvasElement).findByRole("list", {name: "Rules"})).getAllByRole("listitem")

export const Editable: Story = {
    play: async ({canvasElement, args}) => {
        const cards = await ruleCards(canvasElement)
        const canvas = within(canvasElement)
        expect(cards).toHaveLength(8)
        await expect(cards[0]).toHaveTextContent(
            /^1When.*Already enrolled.*and.*At most 1 detail differs.*Then.*Reject.*The voter is told: “Already approved”\./
        )
        await expect(cards[2]).toHaveTextContent(
            /^3When.*All details match.*Then.*Approve automatically$/
        )
        await expect(canvas.getByRole("button", {name: "Move rule 1 up"})).toBeDisabled()
        await expect(canvas.getByRole("button", {name: "Move rule 7 down"})).toBeDisabled()
        // The last rule can only be edited.
        expect(within(cards[7]).getAllByRole("button")).toHaveLength(1)

        await userEvent.click(canvas.getByRole("button", {name: "Edit rule 2"}))
        expect(args.onEdit).toHaveBeenLastCalledWith(1)
        await userEvent.click(canvas.getByRole("button", {name: "Edit the last rule"}))
        expect(args.onEdit).toHaveBeenLastCalledWith(null)
        await userEvent.click(canvas.getByRole("button", {name: "Move rule 3 up"}))
        expect(args.onMove).toHaveBeenLastCalledWith(2, -1)
        await userEvent.click(canvas.getByRole("button", {name: "Move rule 3 down"}))
        expect(args.onMove).toHaveBeenLastCalledWith(2, 1)
        await userEvent.click(canvas.getByRole("button", {name: "Delete rule 7"}))
        expect(args.onDelete).toHaveBeenLastCalledWith(6)
        await userEvent.click(canvas.getByRole("button", {name: "Add rule"}))
        expect(args.onAdd).toHaveBeenCalledTimes(1)
    },
}

export const ReadOnly: Story = {
    args: {canEdit: false},
    play: async ({canvasElement}) => {
        const cards = await ruleCards(canvasElement)
        expect(cards).toHaveLength(8)
        await expect(cards[7]).toHaveTextContent(
            /Otherwise.*None of the rules above apply.*Then.*Reject/
        )
        expect(within(canvasElement).queryByRole("button")).toBeNull()
    },
}

export const Association: Story = {
    args: {matrix: "association"},
    play: async ({canvasElement}) => {
        const cards = await ruleCards(canvasElement)
        expect(cards).toHaveLength(4)
        await expect(cards[1]).toHaveTextContent(
            /^2When.*Identity typed by hand.*Send to a person.*“Identity not verified”/
        )
        await expect(cards[3]).toHaveTextContent(/Otherwise.*Send to a person.*“No matching voter”/)
    },
}

export const RulesAnExampleAndAnEnrollmentPointAt: Story = {
    args: {appliesTo: 3, cameFrom: 5},
    play: async ({canvasElement}) => {
        const cards = await ruleCards(canvasElement)
        await expect(within(cards[2]).getByText("Applies to your example")).toBeVisible()
        await expect(
            within(cards[4]).getByText("Decided the enrollment you came from")
        ).toBeVisible()
        expect(cards.map((card) => card.getAttribute("data-highlighted"))).toEqual([
            "false",
            "false",
            "true",
            "false",
            "true",
            "false",
            "false",
            "false",
        ])
    },
}

export const TheLastRuleDecides: Story = {
    args: {appliesTo: null, cameFrom: null, canEdit: false},
    play: async ({canvasElement}) => {
        const cards = await ruleCards(canvasElement)
        await expect(within(cards[7]).getByText("Applies to your example")).toBeVisible()
        await expect(
            within(cards[7]).getByText("Decided the enrollment you came from")
        ).toBeVisible()
        await expect(cards[7]).toHaveAttribute("data-highlighted", "true")
        await expect(cards[0]).toHaveAttribute("data-highlighted", "false")
    },
}

export const RulesThatCannotBeSaved: Story = {
    args: {
        problems: [
            {code: EMatrixError.ACCEPTS_MANUAL_ENTRY, rule: 2},
            {code: EMatrixError.NO_CONDITIONS, rule: 2},
            {code: EMatrixError.OTHERWISE_ACCEPTS, rule: null},
            // Shown next to the compared details, not on a rule.
            {code: EMatrixError.NO_COMPARED_FIELDS, rule: null},
        ],
    },
    play: async ({canvasElement}) => {
        const cards = await ruleCards(canvasElement)
        await expect(within(cards[1]).getByRole("alert")).toHaveTextContent(
            "Enrollments whose identity was typed by hand can't be approved automatically." +
                "Add at least one condition. Only the last rule applies to everything else."
        )
        await expect(within(cards[7]).getByRole("alert")).toHaveTextContent(
            /^The last rule can send enrollments to a person or reject them, but not approve them\.$/
        )
        expect(within(canvasElement).getAllByRole("alert")).toHaveLength(2)
    },
}
