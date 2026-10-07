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

const MATRIX = comelecMatrix()

const ruleOf = (editing: Scenario["editing"]): IApprovalRule =>
    editing === "new"
        ? NEW_RULE
        : editing === "otherwise"
          ? {when: {}, then: MATRIX.otherwise}
          : MATRIX.rules[editing - 1]

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
                comparedFields={MATRIX.compared_fields}
                validIds={VALID_IDS}
                fieldLabel={humanizeField}
                onClose={onClose}
            />
        </MatrixScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const dialog = async (name: string) => {
    const element = await within(document.body).findByRole("dialog", {name})
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

type Editor = Awaited<ReturnType<typeof dialog>>

const decide = (editor: Editor, decision: RegExp) =>
    userEvent.click(editor.getByRole("radio", {name: decision}))

const addCondition = async (editor: Editor, condition: string) => {
    await userEvent.click(editor.getByRole("button", {name: "Add condition"}))
    await userEvent.click(await within(document.body).findByRole("menuitem", {name: condition}))
    await waitFor(() => expect(within(document.body).queryByRole("menu")).toBeNull())
}

const chooseOption = async (editor: Editor, field: string, option: string) => {
    await userEvent.click(editor.getByRole("combobox", {name: field}))
    await userEvent.click(await within(document.body).findByRole("option", {name: option}))
    await waitFor(() => expect(within(document.body).queryByRole("listbox")).toBeNull())
}

const pressed = (editor: Editor, group: string) =>
    within(editor.getByRole("group", {name: group}))
        .getAllByRole("button", {pressed: true})
        .map((button) => button.textContent)

export const EditRule: Story = {
    play: async ({args}) => {
        const editor = await dialog("Edit rule 4")
        await expect(editor.getByRole("heading", {name: "Edit rule 4"})).toBeVisible()
        await expect(editor.getByTestId("rule-summary")).toHaveTextContent(
            "When exactly 1 detail differs and embassy differs, approve the enrollment automatically."
        )
        await expect(editor.getByRole("combobox", {name: "Details that differ"})).toHaveTextContent(
            "Exactly 1"
        )
        expect(pressed(editor, "Embassy")).toEqual(["Different"])
        await expect(editor.getByRole("radio", {name: /^Approve automatically/})).toBeChecked()
        // An approval tells the voter nothing.
        expect(editor.queryByRole("combobox", {name: "What the voter is told"})).toBeNull()

        await decide(editor, /^Send to a person/)
        await expect(
            await editor.findByRole("combobox", {name: "What the voter is told"})
        ).toHaveTextContent("No matching voter")
        await expect(
            editor.getByText(
                /^We couldn't find a voter in the registry that matches your details\./
            )
        ).toBeVisible()
        await addCondition(editor, "ID type")
        await expect(editor.getByRole("combobox", {name: "ID type"})).toHaveTextContent(
            "Passport"
        )
        await expect(editor.getByTestId("rule-summary")).toHaveTextContent(
            "When iD: Passport and exactly 1 detail differs and embassy differs, send the enrollment to a person."
        )
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

export const ChangeAndRemoveConditions: Story = {
    play: async ({args}) => {
        const editor = await dialog("Edit rule 4")
        await userEvent.click(
            within(editor.getByRole("group", {name: "Embassy"})).getByRole("button", {name: "Same"})
        )
        expect(pressed(editor, "Embassy")).toEqual(["Same"])
        await chooseOption(editor, "Details that differ", "At most 2")
        await expect(editor.getByTestId("rule-summary")).toHaveTextContent(
            "When at most 2 details differ and embassy matches, approve the enrollment automatically."
        )
        await userEvent.click(editor.getByRole("button", {name: "Remove “Embassy”"}))
        await waitFor(() => expect(editor.queryByRole("group", {name: "Embassy"})).toBeNull())
        await expect(editor.getByTestId("rule-summary")).toHaveTextContent(
            "When at most 2 details differ, approve the enrollment automatically."
        )
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        expect(args.onClose).toHaveBeenLastCalledWith({
            when: {differing: EDifferingFields.AT_MOST_2},
            then: {decision: IApplicationsStatus.ACCEPTED},
        })
    },
}

export const ManualEntryCannotBeApproved: Story = {
    args: {editing: 2},
    play: async ({args}) => {
        const editor = await dialog("Edit rule 2")
        expect(pressed(editor, "Identity check")).toEqual(["Typed by hand"])
        await expect(
            editor.getByRole("combobox", {name: "What the voter is told"})
        ).toHaveTextContent("Identity not verified")
        await decide(editor, /^Approve automatically/)
        // The mistake shows at once, and the rule can't be applied.
        await expect(await editor.findByRole("alert")).toHaveTextContent(
            "Enrollments whose identity was typed by hand can't be approved automatically."
        )
        await expect(editor.getByRole("button", {name: "Apply"})).toBeDisabled()
        expect(editor.queryByRole("combobox", {name: "What the voter is told"})).toBeNull()
        expect(args.onClose).not.toHaveBeenCalled()

        await userEvent.click(editor.getByRole("button", {name: "Close"}))
        expect(args.onClose).toHaveBeenLastCalledWith(null)
    },
}

export const EnrolledVoterCannotBeApproved: Story = {
    args: {editing: 1},
    play: async ({args}) => {
        const editor = await dialog("Edit rule 1")
        expect(pressed(editor, "Already enrolled")).toEqual(["Yes"])
        await decide(editor, /^Approve automatically/)
        await expect(await editor.findByRole("alert")).toHaveTextContent(
            "A voter who is already enrolled can't be approved again."
        )
        await expect(editor.getByRole("button", {name: "Apply"})).toBeDisabled()
        // Saying the voter is not enrolled yet makes the rule valid again.
        await userEvent.click(
            within(editor.getByRole("group", {name: "Already enrolled"})).getByRole("button", {
                name: "No",
            })
        )
        await waitFor(() => expect(editor.queryByRole("alert")).toBeNull())
        await expect(editor.getByRole("button", {name: "Apply"})).toBeEnabled()
        expect(args.onClose).not.toHaveBeenCalled()
    },
}

export const NewRule: Story = {
    args: {editing: "new"},
    play: async ({args}) => {
        const editor = await dialog("New rule")
        await expect(editor.getByTestId("rule-summary")).toHaveTextContent(
            "Add a condition to say when this rule applies."
        )
        // What is still missing shows once the administrator tries to apply the rule.
        expect(editor.queryByRole("alert")).toBeNull()
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        await waitFor(() =>
            expect(editor.getAllByRole("alert").map((alert) => alert.textContent)).toEqual([
                "Add at least one condition. Only the last rule applies to everything else.",
                "Choose what the voter is told.",
            ])
        )
        expect(args.onClose).not.toHaveBeenCalled()

        await addCondition(editor, "Voter in the registry")
        expect(pressed(editor, "Voter in the registry")).toEqual(["Yes"])
        await userEvent.click(
            within(editor.getByRole("group", {name: "Voter in the registry"})).getByRole("button", {
                name: "No",
            })
        )
        await chooseOption(editor, "What the voter is told", "Missing data")
        await waitFor(() => expect(editor.queryByRole("alert")).toBeNull())
        await expect(editor.getByTestId("rule-summary")).toHaveTextContent(
            "When no voter found in the registry, send the enrollment to a person."
        )
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
        const editor = await dialog("Edit the last rule")
        await expect(editor.getByTestId("rule-summary")).toHaveTextContent(
            "If none of the rules above apply, reject the enrollment."
        )
        // The last rule has a decision and no conditions.
        expect(editor.queryByRole("button", {name: "Add condition"})).toBeNull()
        await decide(editor, /^Approve automatically/)
        await expect(await editor.findByRole("alert")).toHaveTextContent(
            "The last rule can send enrollments to a person or reject them, but not approve them."
        )
        await expect(editor.getByRole("button", {name: "Apply"})).toBeDisabled()

        // The reason is kept while Approve was tried.
        await decide(editor, /^Send to a person/)
        await waitFor(() => expect(editor.getByRole("button", {name: "Apply"})).toBeEnabled())
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        expect(args.onClose).toHaveBeenLastCalledWith({
            when: {},
            then: {decision: IApplicationsStatus.PENDING, reason: EMatrixReason.NO_VOTER},
        })
        await userEvent.click(editor.getByRole("button", {name: "Cancel"}))
        expect(args.onClose).toHaveBeenLastCalledWith(null)
    },
}
