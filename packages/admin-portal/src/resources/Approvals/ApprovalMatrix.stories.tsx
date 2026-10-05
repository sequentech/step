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
import {ApprovalMatrix} from "./ApprovalMatrix"
import {EMatrixReason} from "./approvalMatrix"
import {
    MatrixScreen,
    comelecMatrix,
    matrixCalls,
    setUpMatrix,
    unexpectedCalls,
    type MatrixServices,
} from "./__stories__/ApprovalMatrixFixture"

interface Scenario extends MatrixServices {
    /** The rule that decided the enrollment the administrator came from. */
    cameFrom?: {version: number; rule: number | null}
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
    beforeEach: ({args}) => setUpMatrix(args),
    render: ({canEdit, goBack, cameFrom}) => (
        <MatrixScreen canEdit={canEdit}>
            <div data-testid="screen">
                <ApprovalMatrix electionEventId={EVENT_ID} goBack={goBack} cameFrom={cameFrom} />
            </div>
        </MatrixScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** The rule cards in order; the last one is the rule that applies when no other does. */
const ruleCards = async (canvasElement: HTMLElement) =>
    within(await within(canvasElement).findByRole("list", {name: "Rules"})).getAllByRole("listitem")

const compared = (canvasElement: HTMLElement) =>
    within(within(canvasElement).getByRole("group", {name: "What we compare"}))

const example = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("status", {name: "Try an example"})

const unsavedBar = (canvasElement: HTMLElement) =>
    within(canvasElement).queryByRole("region", {name: "Unsaved changes"})

const dialog = async (name: string | RegExp) => {
    const element = await within(document.body).findByRole("dialog", {name})
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

export const BuiltInVersion: Story = {
    play: async ({canvasElement}) => {
        const cards = await ruleCards(canvasElement)
        const canvas = within(canvasElement)
        expect(cards).toHaveLength(8)
        await expect(cards[4]).toHaveTextContent(
            /^5When.*Exactly 1 detail differs.*and.*Embassy matches.*Then.*Send to a person.*The voter is told: “No matching voter”\./
        )
        await expect(cards[7]).toHaveTextContent(
            /Otherwise.*None of the rules above apply.*Then.*Reject.*“No matching voter”/
        )
        await expect(canvas.getByText("Version 1")).toBeVisible()
        await expect(
            canvas.getByText("Built-in rules, used until a version is saved")
        ).toBeVisible()
        for (const detail of [
            "First Name",
            "Middle Name",
            "Last Name",
            "Date of birth",
            "Embassy",
        ]) {
            await expect(
                compared(canvasElement).getByRole("button", {name: detail})
            ).toHaveAttribute("aria-pressed", "true")
        }
        // The example starts from an enrollment whose details all match.
        await waitFor(() => expect(example(canvasElement)).toHaveTextContent("Rule 3 applies"))
        await expect(await within(cards[2]).findByText("Applies to your example")).toBeVisible()
        expect(unsavedBar(canvasElement)).toBeNull()
        expect(unexpectedCalls()).toEqual([])
    },
}

export const AssociationVersion: Story = {
    args: {saved: "association"},
    play: async ({canvasElement}) => {
        const cards = await ruleCards(canvasElement)
        const canvas = within(canvasElement)
        expect(cards).toHaveLength(4)
        await expect(cards[3]).toHaveTextContent(/Otherwise.*Send to a person.*“No matching voter”/)
        await expect(canvas.getByText("Version 3")).toBeVisible()
        await expect(canvas.getByText(/^Saved .* by admin$/)).toBeVisible()
        expect(compared(canvasElement).queryByRole("button", {name: "Embassy"})).toBeNull()
        expect(canvas.queryByRole("group", {name: "Embassy"})).toBeNull()
    },
}

export const ReadOnly: Story = {
    args: {canEdit: false},
    play: async ({canvasElement}) => {
        const cards = await ruleCards(canvasElement)
        const canvas = within(canvasElement)
        expect(cards).toHaveLength(8)
        await expect(canvas.getByText("View only")).toBeVisible()
        await expect(canvas.getByText("You can see the rules but not change them")).toBeVisible()
        await expect(compared(canvasElement).getByRole("button", {name: "Embassy"})).toBeDisabled()
        expect(canvas.queryByRole("button", {name: "Compare another detail"})).toBeNull()
        expect(canvas.queryByRole("button", {name: /rule/})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Add rule"})).toBeNull()
        // An example can still be tried.
        await userEvent.click(
            within(canvas.getByRole("group", {name: "Identity check"})).getByRole("button", {
                name: "Typed by hand",
            })
        )
        await waitFor(() => expect(example(canvasElement)).toHaveTextContent("Rule 2 applies"))
        expect(unsavedBar(canvasElement)).toBeNull()
    },
}

export const EditARuleAndSave: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Edit rule 4"}))
        const editor = await dialog("Edit rule 4")
        await userEvent.click(editor.getByRole("radio", {name: /^Send to a person/}))
        // A rule that stops approving starts from the reason of the last rule.
        await expect(editor.getByTestId("rule-summary")).toHaveTextContent(
            "When exactly 1 detail differs and embassy differs, send the enrollment to a person."
        )
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))

        await waitFor(async () =>
            expect((await ruleCards(canvasElement))[3]).toHaveTextContent(
                /Embassy differs.*Send to a person.*“No matching voter”/
            )
        )
        const bar = within(await canvas.findByRole("region", {name: "Unsaved changes"}))
        await expect(bar.getByText("You have unsaved changes")).toBeVisible()
        await expect(
            bar.getByText("Rule 4: approve automatically → send to a person")
        ).toBeVisible()
        await userEvent.click(bar.getByRole("button", {name: "Save as version 2"}))

        const confirmation = await dialog("Save as version 2?")
        await expect(confirmation.getByText("What changed")).toBeVisible()
        await expect(confirmation.getAllByRole("listitem")).toHaveLength(1)
        await expect(confirmation.getByRole("listitem")).toHaveTextContent(
            "Rule 4: approve automatically → send to a person"
        )
        await userEvent.click(confirmation.getByRole("button", {name: "Save version 2"}))

        await expect(await canvas.findByText("Version 2")).toBeVisible()
        await expect(canvas.getByText(/^Saved .* by admin$/)).toBeVisible()
        const saved = await within(document.body).findByText("Saved as version 2")
        await waitFor(() => expect(saved).toBeVisible())
        const expected = comelecMatrix()
        expected.rules[3].then = {
            decision: IApplicationsStatus.PENDING,
            reason: EMatrixReason.NO_VOTER,
        }
        expect(matrixCalls("SaveApprovalMatrix")).toHaveLength(1)
        expect(matrixCalls("SaveApprovalMatrix")[0].variables).toEqual({
            electionEventId: EVENT_ID,
            matrix: expected,
        })
        await waitFor(() => expect(unsavedBar(canvasElement)).toBeNull())
    },
}

export const ReorderAddDeleteAndDiscard: Story = {
    args: {saved: "association"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Move rule 1 down"}))
        await waitFor(async () =>
            expect((await ruleCards(canvasElement))[0]).toHaveTextContent(
                /^1When.*Identity typed by hand/
            )
        )
        const bar = within(await canvas.findByRole("region", {name: "Unsaved changes"}))
        await expect(bar.getByText("Rules were reordered")).toBeVisible()

        await userEvent.click(canvas.getByRole("button", {name: "Delete rule 3"}))
        await waitFor(async () => expect(await ruleCards(canvasElement)).toHaveLength(3))

        await userEvent.click(canvas.getByRole("button", {name: "Add rule"}))
        const editor = await dialog("New rule")
        await expect(editor.getByTestId("rule-summary")).toHaveTextContent(
            "Add a condition to say when this rule applies."
        )
        await userEvent.click(editor.getByRole("button", {name: "Add condition"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitem", {name: "Voter in the registry"})
        )
        await userEvent.click(
            within(await editor.findByRole("group", {name: "Voter in the registry"})).getByRole(
                "button",
                {name: "No"}
            )
        )
        await userEvent.click(editor.getByRole("combobox", {name: "What the voter is told"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "Missing data"})
        )
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        await waitFor(async () =>
            expect((await ruleCards(canvasElement))[2]).toHaveTextContent(
                /^3When.*No voter found in the registry.*Send to a person.*“Missing data”/
            )
        )
        await expect(bar.getByRole("button", {name: "Save as version 4"})).toBeEnabled()

        await userEvent.click(bar.getByRole("button", {name: "Discard changes"}))
        await waitFor(() => expect(unsavedBar(canvasElement)).toBeNull())
        const cards = await ruleCards(canvasElement)
        expect(cards).toHaveLength(4)
        await expect(cards[0]).toHaveTextContent(/^1When.*Already enrolled/)
        expect(matrixCalls("SaveApprovalMatrix")).toEqual([])
    },
}

export const ChangeTheComparedDetails: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await expect(canvas.getByRole("group", {name: "Embassy"})).toBeVisible()
        await userEvent.click(compared(canvasElement).getByRole("button", {name: "Embassy"}))

        await expect(
            compared(canvasElement).getByRole("button", {name: "Embassy"})
        ).toHaveAttribute("aria-pressed", "false")
        // The conditions on a detail that is no longer compared go away.
        await waitFor(async () =>
            expect((await ruleCards(canvasElement))[3]).toHaveTextContent(
                /^4When.*Exactly 1 detail differs.*Then.*Approve automatically$/
            )
        )
        await waitFor(() => expect(canvas.queryByRole("group", {name: "Embassy"})).toBeNull())
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
        const bar = within(await canvas.findByRole("region", {name: "Unsaved changes"}))
        await expect(bar.getByText(/^The details compared changed/)).toBeVisible()

        // A detail of the voter's profile that wasn't compared yet.
        await userEvent.click(canvas.getByRole("button", {name: "Compare another detail"}))
        await userEvent.click(await within(document.body).findByRole("menuitem", {name: "Email"}))
        await expect(
            await compared(canvasElement).findByRole("button", {name: "Email"})
        ).toHaveAttribute("aria-pressed", "true")
        await expect(await canvas.findByRole("group", {name: "Email"})).toBeVisible()
        await waitFor(() =>
            expect(matrixCalls("EvaluateApprovalMatrix").at(-1)?.variables.matrix).toMatchObject({
                compared_fields: ["firstName", "middleName", "lastName", "dateOfBirth", "email"],
            })
        )
    },
}

export const ARuleThatCannotBeSaved: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Edit the last rule"}))
        const editor = await dialog("Edit the last rule")
        await userEvent.click(editor.getByRole("radio", {name: /^Approve automatically/}))
        await expect(await editor.findByRole("alert")).toHaveTextContent(
            "The last rule can send enrollments to a person or reject them, but not approve them."
        )
        await expect(editor.getByRole("button", {name: "Apply"})).toBeDisabled()
        await userEvent.click(editor.getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        expect(unsavedBar(canvasElement)).toBeNull()
        await expect((await ruleCards(canvasElement))[7]).toHaveTextContent(/Reject/)
    },
}

export const NothingToCompare: Story = {
    args: {saved: "association"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        for (const detail of ["First Name", "Last Name", "Date of birth"]) {
            await userEvent.click(compared(canvasElement).getByRole("button", {name: detail}))
        }
        await expect(await canvas.findByRole("alert")).toHaveTextContent(
            "Choose at least one detail to compare with the registry."
        )
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent(
                "Fix these rules to try an example:Choose at least one detail to compare with the registry."
            )
        )
        const bar = within(await canvas.findByRole("region", {name: "Unsaved changes"}))
        await expect(bar.getByRole("button", {name: "Save as version 4"})).toBeDisabled()

        await userEvent.click(compared(canvasElement).getByRole("button", {name: "Last Name"}))
        await waitFor(() =>
            expect(bar.getByRole("button", {name: "Save as version 4"})).toBeEnabled()
        )
        expect(canvas.queryByRole("alert")).toBeNull()
    },
}

export const SaveFails: Story = {
    args: {saves: "error"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Delete rule 7"}))
        const bar = within(await canvas.findByRole("region", {name: "Unsaved changes"}))
        await expect(
            bar.getByText(
                "A rule was removed (Exactly 2 details differ, Middle Name differs, Last Name differs)"
            )
        ).toBeVisible()
        await userEvent.click(bar.getByRole("button", {name: "Save as version 2"}))
        const confirmation = await dialog("Save as version 2?")
        await userEvent.click(confirmation.getByRole("button", {name: "Save version 2"}))
        const failure = await within(document.body).findByText(
            "The approval matrix could not be saved"
        )
        await waitFor(() => expect(failure).toBeVisible())
        await expect(bar.getByText("You have unsaved changes")).toBeVisible()
        await expect(canvas.getByText("Version 1")).toBeVisible()
    },
}

export const OpenedFromAnEnrollment: Story = {
    args: {cameFrom: {version: 1, rule: 5}},
    play: async ({canvasElement, args}) => {
        const cards = await ruleCards(canvasElement)
        await expect(
            within(cards[4]).getByText("Decided the enrollment you came from")
        ).toBeVisible()
        await expect(cards[4]).toHaveAttribute("data-highlighted", "true")
        expect(
            within(canvasElement).getAllByText("Decided the enrollment you came from")
        ).toHaveLength(1)
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Approvals"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const OpenedFromAnEnrollmentOfAnOlderVersion: Story = {
    args: {saved: "association", cameFrom: {version: 1, rule: 5}},
    play: async ({canvasElement}) => {
        await ruleCards(canvasElement)
        // The rules on screen are not the ones that decided the enrollment.
        expect(within(canvasElement).queryByText("Decided the enrollment you came from")).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("progressbar", {name: "Approval matrix"})
        ).toBeVisible()
    },
}

export const LoadFails: Story = {
    args: {reads: "error"},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("alert")).toHaveTextContent(
            "The approval matrix could not be loaded."
        )
        await userEvent.click(canvas.getByRole("button", {name: "Approvals"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const NarrowScreen: Story = {
    globals: {viewport: {value: "iphone5", isRotated: false}},
    play: async ({canvasElement}) => {
        const cards = await ruleCards(canvasElement)
        await expect(cards[0]).toBeVisible()
        const screen = within(canvasElement).getByTestId("screen")
        expect(window.innerWidth).toBe(320)
        expect(screen.scrollWidth).toBeLessThanOrEqual(screen.clientWidth)
        await expect(
            await within(canvasElement).findByRole("group", {name: "Embassy"})
        ).toBeVisible()
    },
}
