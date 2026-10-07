// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Mock} from "storybook/test"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IApplicationsStatus} from "@/types/applications"
import {ApprovalMatrixTest} from "./ApprovalMatrixTest"
import type {RulePointer} from "./ApprovalMatrixRules"
import {EFieldMatch, EIdentityMethod, humanizeField} from "./approvalMatrix"
import {
    MatrixScreen,
    VALID_IDS,
    associationMatrix,
    comelecMatrix,
    matrixCalls,
    setUpMatrix,
} from "./__stories__/ApprovalMatrixFixture"

interface Scenario {
    matrix: "comelec" | "association" | "invalid"
    /** Trying the example. */
    evaluates: "result" | "error"
    onResult: Mock<(rule: RulePointer) => void>
}

const matrixOf = (name: Scenario["matrix"]) => {
    if (name === "association") return associationMatrix()
    const matrix = comelecMatrix()
    if (name === "invalid") matrix.rules[1].then = {decision: IApplicationsStatus.ACCEPTED}
    return matrix
}

// The panel tries the example again whenever it is given another matrix.
const MATRICES = {
    comelec: matrixOf("comelec"),
    association: matrixOf("association"),
    invalid: matrixOf("invalid"),
}

const meta = {
    title: "Admin/Approvals/ApprovalMatrixTest",
    component: ApprovalMatrixTest,
    args: {matrix: "comelec", evaluates: "result", onResult: fn()},
    argTypes: {
        matrix: {control: "inline-radio", options: ["comelec", "association", "invalid"]},
        evaluates: {control: "inline-radio", options: ["result", "error"]},
    },
    beforeEach: ({args}) => setUpMatrix({evaluates: args.evaluates}),
    render: ({matrix, onResult}) => (
        <MatrixScreen canEdit={false}>
            <ApprovalMatrixTest
                electionEventId={EVENT_ID}
                matrix={MATRICES[matrix]}
                validIds={VALID_IDS}
                fieldLabel={humanizeField}
                onResult={onResult}
            />
        </MatrixScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const choices = (canvasElement: HTMLElement, label: string) =>
    within(within(canvasElement).getByRole("group", {name: label}))

const choose = (canvasElement: HTMLElement, label: string, option: string) =>
    userEvent.click(choices(canvasElement, label).getByRole("button", {name: option}))

const outcome = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("status", {name: "Try an example"})

const lastEnrollment = () => matrixCalls("EvaluateApprovalMatrix").at(-1)?.variables.enrollment

export const EverythingMatches: Story = {
    play: async ({canvasElement, args}) => {
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                /^Rule 3 applies.*Approve automatically$/
            )
        )
        await expect(
            choices(canvasElement, "Identity check").getByRole("button", {
                name: "Verified by ID scan",
            })
        ).toHaveAttribute("aria-pressed", "true")
        await expect(
            choices(canvasElement, "Embassy").getByRole("button", {name: "Same"})
        ).toHaveAttribute("aria-pressed", "true")
        expect(matrixCalls("EvaluateApprovalMatrix")[0].variables).toMatchObject({
            electionEventId: EVENT_ID,
            enrollment: {
                identity: EIdentityMethod.VERIFIED,
                voter_found: true,
                already_enrolled: false,
                valid_id: null,
                fields: {embassy: EFieldMatch.MATCHES},
            },
        })
        await waitFor(() => expect(args.onResult).toHaveBeenLastCalledWith(3))
    },
}

export const DescribeAnEnrollment: Story = {
    play: async ({canvasElement, args}) => {
        await choose(canvasElement, "First Name", "Different")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                /^Rule 5 applies.*Send to a person.*We couldn't find a voter in the registry that matches your details\./
            )
        )
        await waitFor(() => expect(args.onResult).toHaveBeenLastCalledWith(5))
        await choose(canvasElement, "Last Name", "Different")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(/^The last rule applies.*Reject/)
        )
        await waitFor(() => expect(args.onResult).toHaveBeenLastCalledWith(null))
        await choose(canvasElement, "Identity check", "Typed by hand")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                /^Rule 2 applies.*Send to a person.*We could not verify your identity automatically/
            )
        )
        expect(lastEnrollment()).toMatchObject({
            identity: EIdentityMethod.MANUAL_ENTRY,
            fields: {firstName: EFieldMatch.DIFFERS, lastName: EFieldMatch.DIFFERS},
        })
    },
}

export const AlreadyEnrolledWithAnIdType: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await choose(canvasElement, "Already enrolled", "Yes")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                /^Rule 1 applies.*Reject.*You are already enrolled\./
            )
        )
        await expect(canvas.getByRole("combobox", {name: "ID type"})).toHaveTextContent(
            "Not reported"
        )
        await userEvent.click(canvas.getByRole("combobox", {name: "ID type"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Passport"}))
        await waitFor(() =>
            expect(lastEnrollment()).toMatchObject({
                already_enrolled: true,
                valid_id: "philippinePassport",
            })
        )
    },
}

export const NoVoterInTheRegistry: Story = {
    play: async ({canvasElement}) => {
        await choose(canvasElement, "Voter in the registry", "No")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(/^The last rule applies.*Reject/)
        )
        const canvas = within(canvasElement)
        // Nothing is compared, and nobody can be already enrolled, without a voter.
        expect(canvas.queryByRole("group", {name: "First Name"})).toBeNull()
        expect(canvas.queryByText("Details compared")).toBeNull()
        for (const button of choices(canvasElement, "Already enrolled").getAllByRole("button")) {
            await expect(button).toBeDisabled()
        }
        expect(lastEnrollment()).toMatchObject({voter_found: false, already_enrolled: false})
    },
}

export const Association: Story = {
    args: {matrix: "association"},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByRole("group", {name: "Embassy"})).toBeNull()
        await choose(canvasElement, "Date Of Birth", "Different")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                /^The last rule applies.*Send to a person/
            )
        )
    },
}

export const RulesThatCannotBeSaved: Story = {
    args: {matrix: "invalid"},
    play: async ({canvasElement, args}) => {
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                "Fix these rules to try an example:" +
                    "Rule 2: Enrollments whose identity was typed by hand can't be approved automatically."
            )
        )
        // No rule decides the example.
        expect(args.onResult.mock.calls.every(([rule]) => rule === undefined)).toBe(true)
    },
}

export const TryingFails: Story = {
    args: {evaluates: "error"},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent("The example could not be tried.")
        )
    },
}
