// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IApplicationsStatus} from "@/types/applications"
import {ApprovalMatrixTest} from "./ApprovalMatrixTest"
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
}

const matrixOf = (name: Scenario["matrix"]) => {
    if (name === "association") return associationMatrix()
    const matrix = comelecMatrix()
    if (name === "invalid") matrix.rules[1].then = {decision: IApplicationsStatus.ACCEPTED}
    return matrix
}

const meta = {
    title: "Admin/Approvals/ApprovalMatrixTest",
    component: ApprovalMatrixTest,
    args: {matrix: "comelec"},
    argTypes: {matrix: {control: "inline-radio", options: ["comelec", "association", "invalid"]}},
    parameters: {
        expectedFailure: {
            reason: "The status chips put white text on light colours.",
            a11y: ["color-contrast"],
        },
    },
    beforeEach: () => setUpMatrix(),
    render: ({matrix}) => (
        <MatrixScreen canEdit={false}>
            <ApprovalMatrixTest
                electionEventId={EVENT_ID}
                matrix={matrixOf(matrix)}
                validIds={VALID_IDS}
                fieldLabel={humanizeField}
            />
        </MatrixScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const choose = async (canvasElement: HTMLElement, field: string, option: string) => {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: field}))
    await userEvent.click(await within(document.body).findByRole("option", {name: option}))
}

const outcome = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("status", {name: "Test The Matrix"})

export const EverythingMatches: Story = {
    play: async ({canvasElement}) => {
        await waitFor(() => expect(outcome(canvasElement)).toHaveTextContent("Rule 3 applies:"))
        await expect(outcome(canvasElement)).toHaveTextContent("ACCEPTED")
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
    },
}

export const DescribeAnEnrollment: Story = {
    play: async ({canvasElement}) => {
        await choose(canvasElement, "First Name", "Differs")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                "Rule 5 applies:PENDINGNo Matching Voter"
            )
        )
        await choose(canvasElement, "Last Name", "Differs")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                "No rule applies, so the last rule (Otherwise) decides:REJECTEDNo Matching Voter"
            )
        )
        await choose(canvasElement, "Identity Verification", "Entered manually")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                "Rule 2 applies:PENDINGIdentity Not Verified"
            )
        )
    },
}

export const NoVoterInTheRegistry: Story = {
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await choose(canvasElement, "Voter Found In Registry", "No")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                "No rule applies, so the last rule (Otherwise) decides:REJECTED"
            )
        )
        const canvas = within(canvasElement)
        expect(canvas.queryByRole("combobox", {name: "First Name"})).toBeNull()
        await expect(
            canvas.getByRole("combobox", {name: "Voter Already Enrolled"})
        ).toHaveAttribute("aria-disabled", "true")
    },
}

export const Association: Story = {
    args: {matrix: "association"},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByRole("combobox", {name: "Embassy"})).toBeNull()
        await choose(canvasElement, "Date Of Birth", "Differs")
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                "No rule applies, so the last rule (Otherwise) decides:PENDING"
            )
        )
    },
}

export const RulesThatCannotBeSaved: Story = {
    args: {matrix: "invalid"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                "Rule 2: Enrollments whose identity was entered manually can't be approved automatically."
            )
        )
    },
}
