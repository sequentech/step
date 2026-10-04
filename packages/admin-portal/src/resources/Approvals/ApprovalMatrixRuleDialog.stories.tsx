// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Mock} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IApplicationsStatus} from "@/types/applications"
import {ApprovalMatrixRuleDialog} from "./ApprovalMatrixRuleDialog"
import {
    EDifferingFields,
    EFieldMatch,
    EMatrixReason,
    humanizeField,
    type IApprovalRule,
} from "./approvalMatrix"
import {
    MatrixScreen,
    VALID_IDS,
    comelecMatrix,
    setUpMatrix,
} from "./__stories__/ApprovalMatrixFixture"

interface Scenario {
    /** The rule of the built-in matrix being edited, a new rule, or the last rule. */
    editing: number | "new" | "otherwise"
    onClose: Mock<(rule: IApprovalRule | null) => void>
}

const NEW_RULE: IApprovalRule = {when: {}, then: {decision: IApplicationsStatus.PENDING}}

const ruleOf = (editing: Scenario["editing"]): IApprovalRule =>
    editing === "new"
        ? NEW_RULE
        : editing === "otherwise"
          ? {when: {}, then: comelecMatrix().otherwise}
          : comelecMatrix().rules[editing - 1]

const meta = {
    title: "Admin/Approvals/ApprovalMatrixRuleDialog",
    component: ApprovalMatrixRuleDialog,
    args: {editing: 4, onClose: fn()},
    argTypes: {editing: {control: "select", options: [1, 2, 3, 4, 5, 6, 7, "new", "otherwise"]}},
    beforeEach: () => setUpMatrix(),
    render: ({editing, onClose}) => (
        <MatrixScreen canEdit={true}>
            <ApprovalMatrixRuleDialog
                open={true}
                rule={ruleOf(editing)}
                number={typeof editing === "number" ? editing : null}
                isOtherwise={editing === "otherwise"}
                comparedFields={comelecMatrix().compared_fields}
                validIds={VALID_IDS}
                fieldLabel={humanizeField}
                onClose={onClose}
            />
        </MatrixScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const dialog = async () => {
    const element = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

const choose = async (
    editor: Awaited<ReturnType<typeof dialog>>,
    field: string,
    option: string
) => {
    await userEvent.click(editor.getByRole("combobox", {name: field}))
    await userEvent.click(await within(document.body).findByRole("option", {name: option}))
}

export const EditRule: Story = {
    play: async ({args}) => {
        const editor = await dialog()
        await expect(editor.getByRole("heading", {name: "Edit Rule 4"})).toBeVisible()
        await expect(editor.getByRole("combobox", {name: "Fields That Differ"})).toHaveTextContent(
            "Exactly 1"
        )
        await expect(editor.getByRole("combobox", {name: "Embassy"})).toHaveTextContent("Differs")
        await choose(editor, "Decision", "Send to manual review")
        await choose(editor, "Reason Shown To The Voter", "No Matching Voter")
        await choose(editor, "Valid ID", "Philippine Passport")
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        expect(args.onClose).toHaveBeenLastCalledWith({
            when: {
                valid_id: "philippinePassport",
                differing: EDifferingFields.EXACTLY_1,
                fields: {embassy: EFieldMatch.DIFFERS},
            },
            then: {decision: IApplicationsStatus.PENDING, reason: EMatrixReason.NO_VOTER},
        })
    },
}

export const ManualEntryCannotBeApproved: Story = {
    args: {editing: 2},
    play: async ({args}) => {
        const editor = await dialog()
        await choose(editor, "Decision", "Approve automatically")
        await expect(
            editor.getByRole("combobox", {name: "Reason Shown To The Voter"})
        ).toHaveAttribute("aria-disabled", "true")
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        await expect(
            await editor.findByText(
                "Enrollments whose identity was entered manually can't be approved automatically."
            )
        ).toBeVisible()
        expect(args.onClose).not.toHaveBeenCalled()
    },
}

export const EnrolledVoterCannotBeApproved: Story = {
    args: {editing: 1},
    play: async ({args}) => {
        const editor = await dialog()
        await choose(editor, "Decision", "Approve automatically")
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        await expect(
            await editor.findByText("A voter who is already enrolled can't be approved again.")
        ).toBeVisible()
        expect(args.onClose).not.toHaveBeenCalled()
    },
}

export const NewRule: Story = {
    args: {editing: "new"},
    play: async ({args}) => {
        const editor = await dialog()
        await expect(editor.getByRole("heading", {name: "New Rule"})).toBeVisible()
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        await expect(await editor.findByText("Choose the reason shown to the voter.")).toBeVisible()
        await choose(editor, "Voter Found In Registry", "No")
        await choose(editor, "Reason Shown To The Voter", "Missing Data")
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        expect(args.onClose).toHaveBeenLastCalledWith({
            when: {voter_found: false},
            then: {
                decision: IApplicationsStatus.PENDING,
                reason: EMatrixReason.INSUFFICIENT_INFORMATION,
            },
        })
    },
}

export const LastRule: Story = {
    args: {editing: "otherwise"},
    play: async ({args}) => {
        const editor = await dialog()
        await expect(
            editor.getByRole("heading", {name: "Edit The Last Rule (Otherwise)"})
        ).toBeVisible()
        expect(editor.queryByRole("combobox", {name: "Identity Verification"})).toBeNull()
        await choose(editor, "Decision", "Approve automatically")
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        await expect(
            await editor.findByText(
                "The last rule can send enrollments to manual review or reject them, not approve them."
            )
        ).toBeVisible()
        await userEvent.click(editor.getByRole("button", {name: "Cancel"}))
        expect(args.onClose).toHaveBeenLastCalledWith(null)
    },
}
