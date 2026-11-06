// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Mock} from "storybook/test"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IApplicationsStatus} from "@/types/applications"
import {ApprovalMatrixTest, decidingRule, useExampleEvaluation} from "./ApprovalMatrixTest"
import {EFieldMatch, EIdentityMethod, ITestEnrollment, humanizeField} from "./approvalMatrix"
import {EMatrixEntry, recordedDecision, sampleEnrollment} from "./approvalExample"
import {IApplication, applicantName} from "./approvalReview"
import {decisionRecord} from "./__stories__/ApprovalsFixture"
import {
    MatrixScreen,
    VALID_IDS,
    associationMatrix,
    comelecMatrix,
    matrixCalls,
    setUpMatrix,
} from "./__stories__/ApprovalMatrixFixture"

const {PENDING} = IApplicationsStatus
const ID_CARD_TYPE = "sequent.read-only.id-card-type"

/** Carol typed her details by hand. */
const carol = (overrides: Record<string, unknown> = {}, data: Record<string, string> = {}) => ({
    status: PENDING,
    applicant_data: {firstName: "Carol", lastName: "Sample", [ID_CARD_TYPE]: "iBP", ...data},
    annotations: {
        decision: {
            ...decisionRecord({
                rule: 2,
                conditions: {identity: "MANUAL_ENTRY"},
                decision: PENDING,
                reason: "IDENTITY_NOT_VERIFIED",
                identity: "MANUAL_ENTRY",
            }),
            ...overrides,
        },
    },
})

const SOURCES: Record<string, IApplication | undefined> = {
    "none": undefined,
    "carol": carol(),
    // A seaman's book gives first and middle name together.
    "carol-seaman": carol(
        {
            inputs: {
                identity: "MANUAL_ENTRY",
                voter_found: true,
                already_enrolled: false,
                valid_id: "seamanBook",
                fields: {
                    firstName: "MATCHES",
                    lastName: "MATCHES",
                    dateOfBirth: "MATCHES",
                    embassy: "MATCHES",
                },
            },
        },
        {[ID_CARD_TYPE]: "seamanBook"}
    ),
    "carol-unreadable": carol({inputs: {identity: "MANUAL_ENTRY", voter_found: "yes"}}),
    "carol-approved": {...carol(), status: IApplicationsStatus.ACCEPTED},
    "long-name": carol(
        {},
        {
            firstName: "Maximiliana-Bartholomea-Wilhelmina",
            lastName: "Featherstonehaugh-Montgomery-Abernathy-Delacroix",
        }
    ),
}

interface Scenario {
    matrix: "comelec" | "association" | "invalid"
    evaluates: "result" | "error" | "pending"
    source: string
    onBack: Mock<() => void>
}

const matrixOf = (name: Scenario["matrix"]) => {
    if (name === "association") return associationMatrix()
    const matrix = comelecMatrix()
    if (name === "invalid") matrix.rules[1].then = {decision: IApplicationsStatus.ACCEPTED}
    return matrix
}

const MATRICES = {
    comelec: matrixOf("comelec"),
    association: matrixOf("association"),
    invalid: matrixOf("invalid"),
}

/** The panel as the matrix screen uses it: it owns the applied example and decides it. */
function Panel({
    matrix,
    application,
    onBack,
}: {
    matrix: Scenario["matrix"]
    application?: IApplication
    onBack: () => void
}) {
    const recorded = application ? recordedDecision(application) : null
    const entry = recorded?.enrollment ? EMatrixEntry.ENROLLMENT : EMatrixEntry.TOOLBAR
    const joint = entry === EMatrixEntry.ENROLLMENT && Boolean(recorded?.jointNames)
    const [applied, setApplied] = useState<ITestEnrollment>(
        () => recorded?.enrollment ?? sampleEnrollment()
    )
    const evaluation = useExampleEvaluation(EVENT_ID, MATRICES[matrix], applied, entry, joint)
    return (
        <div data-testid="panel" data-rule={String(decidingRule(evaluation))}>
            <ApprovalMatrixTest
                applied={applied}
                onApply={setApplied}
                entry={entry}
                joint={joint}
                source={
                    application && {
                        name: applicantName(application),
                        status: application.status,
                        recorded,
                        onBack,
                    }
                }
                comparedFields={MATRICES[matrix].compared_fields}
                validIds={VALID_IDS}
                fieldLabel={humanizeField}
                evaluation={evaluation}
                unsavedRules={false}
            />
        </div>
    )
}

const meta = {
    title: "Admin/Approvals/ApprovalMatrixTest",
    component: ApprovalMatrixTest,
    args: {matrix: "comelec", evaluates: "result", source: "none", onBack: fn()},
    argTypes: {
        matrix: {control: "inline-radio", options: ["comelec", "association", "invalid"]},
        evaluates: {control: "inline-radio", options: ["result", "error", "pending"]},
        source: {control: "select", options: Object.keys(SOURCES)},
    },
    beforeEach: ({args}) => setUpMatrix({evaluates: args.evaluates}),
    render: ({matrix, source, onBack}) => (
        <MatrixScreen canEdit={false}>
            <div style={{maxWidth: "360px"}}>
                <Panel matrix={matrix} application={SOURCES[source]} onBack={onBack} />
            </div>
        </MatrixScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const canvasOf = (canvasElement: HTMLElement) => within(canvasElement)

const choose = (canvasElement: HTMLElement, label: string, option: string) =>
    userEvent.click(
        within(canvasOf(canvasElement).getByRole("group", {name: label})).getByRole("button", {
            name: option,
        })
    )

const outcome = (canvasElement: HTMLElement) =>
    canvasOf(canvasElement).getByRole("status", {name: "Current simulation"})

const rule = (canvasElement: HTMLElement) =>
    canvasOf(canvasElement).getByTestId("panel").getAttribute("data-rule")

/** The applied example's summary: each label with its value. */
const summary = (canvasElement: HTMLElement) =>
    Object.fromEntries(
        Array.from(canvasElement.querySelectorAll("dt")).map((term) => [
            term.textContent,
            term.nextElementSibling?.textContent,
        ])
    )

const lastEnrollment = () =>
    matrixCalls("EvaluateApprovalMatrix").at(-1)?.variables.enrollment as ITestEnrollment

const edit = (canvasElement: HTMLElement) =>
    userEvent.click(
        canvasOf(canvasElement).getByRole("button", {name: /^(Edit example|Try other details)$/})
    )

const draft = (canvasElement: HTMLElement) =>
    canvasOf(canvasElement).getByRole("group", {
        name: /^(Editing the example|Trying other details) — not/,
    })

const pressed = (group: HTMLElement) => group.querySelector("[aria-pressed='true']")?.textContent

export const SampleEnrollment: Story = {
    play: async ({canvasElement}) => {
        const canvas = canvasOf(canvasElement)
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                /^In the simulation, rule 3 decides.*Approve automatically$/
            )
        )
        expect(rule(canvasElement)).toBe("3")
        await expect(canvas.getByText("Sample enrollment")).toBeVisible()
        await expect(
            canvas.getByText(
                "A verified voter in the registry, not enrolled yet, whose compared details are all the same."
            )
        ).toBeVisible()
        await expect(
            canvas.getByText("Trying an example changes no enrollment and saves nothing.")
        ).toBeVisible()
        expect(summary(canvasElement)).toMatchObject({
            "Identity check": "Verified by ID scan",
            "Voter in the registry": "Yes",
            "ID type": "Not reported",
            "Embassy": "Same",
        })
        expect(canvas.queryByRole("button", {name: /^Back to the enrollment/})).toBeNull()
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

export const DraftInputsWaitForApply: Story = {
    play: async ({canvasElement}) => {
        const canvas = canvasOf(canvasElement)
        await waitFor(() => expect(rule(canvasElement)).toBe("3"))
        const calls = matrixCalls("EvaluateApprovalMatrix").length
        await edit(canvasElement)
        await expect(draft(canvasElement)).toBeVisible()
        // The details in use stay above the editor, under their own heading.
        await expect(
            canvas.getByRole("heading", {level: 4, name: "Sample enrollment"})
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Apply example"})).toBeDisabled()
        await choose(canvasElement, "First Name", "Different")
        await choose(canvasElement, "Identity check", "Typed by hand")

        // Nothing is decided again; the summary and the result still show the applied example.
        expect(matrixCalls("EvaluateApprovalMatrix")).toHaveLength(calls)
        expect(rule(canvasElement)).toBe("3")
        expect(summary(canvasElement)).toMatchObject({
            "Identity check": "Verified by ID scan",
            "First Name": "Same",
        })
        await expect(outcome(canvasElement)).toHaveTextContent(/^In the simulation, rule 3 decides/)

        await userEvent.click(canvas.getByRole("button", {name: "Apply example"}))
        await waitFor(() => expect(rule(canvasElement)).toBe("2"))
        expect(canvas.queryByRole("group", {name: /^Editing the example/})).toBeNull()
        await waitFor(() =>
            expect(canvas.getByRole("button", {name: "Edit example"})).toHaveFocus()
        )
        expect(summary(canvasElement)).toMatchObject({
            "Identity check": "Typed by hand",
            "First Name": "Different",
        })
        expect(lastEnrollment()).toMatchObject({
            identity: EIdentityMethod.MANUAL_ENTRY,
            fields: {firstName: EFieldMatch.DIFFERS, lastName: EFieldMatch.MATCHES},
        })
        // The default-only description no longer fits.
        await expect(canvas.getByText("Your example")).toBeVisible()
        await expect(canvas.getByText("Started from the sample.")).toBeVisible()
        expect(canvas.queryByText("Sample enrollment")).toBeNull()
    },
}

export const CancelKeepsTheAppliedExample: Story = {
    play: async ({canvasElement}) => {
        const canvas = canvasOf(canvasElement)
        await waitFor(() => expect(rule(canvasElement)).toBe("3"))
        await edit(canvasElement)
        await choose(canvasElement, "Voter in the registry", "No")
        await userEvent.click(canvas.getByRole("button", {name: "Cancel"}))
        expect(rule(canvasElement)).toBe("3")
        expect(summary(canvasElement)).toMatchObject({"Voter in the registry": "Yes"})
        // Editing again starts from the applied example, not the cancelled draft.
        await edit(canvasElement)
        expect(
            pressed(
                within(draft(canvasElement)).getByRole("group", {name: "Voter in the registry"})
            )
        ).toBe("Yes")
        await choose(canvasElement, "Identity check", "Typed by hand")
        await expect(canvas.getByRole("button", {name: "Apply example"})).toBeEnabled()
        await userEvent.click(canvas.getByRole("button", {name: "Fill in the sample"}))
        await expect(canvas.getByRole("button", {name: "Apply example"})).toBeDisabled()
    },
}

export const IdentityNotReportedAndAnIdType: Story = {
    play: async ({canvasElement}) => {
        const canvas = canvasOf(canvasElement)
        await waitFor(() => expect(rule(canvasElement)).toBe("3"))
        await edit(canvasElement)
        await choose(canvasElement, "Identity check", "Not reported")
        await choose(canvasElement, "Already enrolled", "Yes")
        await userEvent.click(canvas.getByRole("combobox", {name: "ID type"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Passport"}))
        // Closing the ID type menu doesn't close the editor.
        await expect(draft(canvasElement)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Apply example"}))
        await waitFor(() => expect(rule(canvasElement)).toBe("1"))
        expect(lastEnrollment()).toMatchObject({
            identity: null,
            already_enrolled: true,
            valid_id: "philippinePassport",
        })
        expect(summary(canvasElement)).toMatchObject({
            "Identity check": "Not reported",
            "ID type": "Passport",
        })
    },
}

export const NoVoterInTheRegistry: Story = {
    play: async ({canvasElement}) => {
        const canvas = canvasOf(canvasElement)
        await waitFor(() => expect(rule(canvasElement)).toBe("3"))
        await edit(canvasElement)
        await choose(canvasElement, "Voter in the registry", "No")
        expect(within(draft(canvasElement)).queryByRole("group", {name: "First Name"})).toBeNull()
        for (const button of Array.from(
            within(draft(canvasElement))
                .getByRole("group", {name: "Already enrolled"})
                .querySelectorAll("button")
        )) {
            await expect(button).toBeDisabled()
        }
        await userEvent.click(canvas.getByRole("button", {name: "Apply example"}))
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                /^In the simulation, no rule fits: the last rule decides.*Reject/
            )
        )
        expect(rule(canvasElement)).toBe("null")
        expect(lastEnrollment()).toMatchObject({voter_found: false, fields: {}})
        expect(summary(canvasElement)["First Name"]).toBeUndefined()
    },
}

export const FromAnEnrollment: Story = {
    args: {source: "carol"},
    play: async ({canvasElement, args}) => {
        const canvas = canvasOf(canvasElement)
        await waitFor(() => expect(rule(canvasElement)).toBe("2"))
        await expect(canvas.getByText("Simulating the enrollment of Carol Sample")).toBeVisible()
        await expect(canvasElement).toHaveTextContent("Rule 2 of version 1 · Send to a person.")
        // The middle name is compared, but Carol's decision didn't record it.
        expect(summary(canvasElement)).toMatchObject({
            "Identity check": "Typed by hand",
            "Middle Name": "Not recorded",
            "Last Name": "Same",
        })
        expect(lastEnrollment().fields).not.toHaveProperty("middleName")
        await expect(
            canvas.getByText(
                "A compared detail wasn't recorded, so the simulation isn't compared with the recorded decision."
            )
        ).toBeVisible()
        expect(canvasElement.textContent).not.toMatch(/same decision|different decision/)

        await edit(canvasElement)
        expect(
            pressed(within(draft(canvasElement)).getByRole("group", {name: "Middle Name"}))
        ).toBeUndefined()
        await expect(
            canvas.getByRole("button", {name: "Fill in the recorded details"})
        ).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Fill in the sample"})).toBeNull()
        await choose(canvasElement, "Middle Name", "Same")
        await userEvent.click(canvas.getByRole("button", {name: "Use in the simulation"}))
        await waitFor(() =>
            expect(
                canvas.getByText(
                    "These aren't the recorded details, so the simulation isn't compared with the recorded decision."
                )
            ).toBeVisible()
        )

        await userEvent.click(
            canvas.getByRole("button", {name: "Back to the enrollment of Carol Sample"})
        )
        expect(args.onBack).toHaveBeenCalledTimes(1)
    },
}

export const FromAnEnrollmentWhoseIdGivesFirstAndMiddleName: Story = {
    args: {source: "carol-seaman"},
    play: async ({canvasElement}) => {
        const canvas = canvasOf(canvasElement)
        await waitFor(() => expect(rule(canvasElement)).toBe("2"))
        expect(summary(canvasElement)).toMatchObject({
            "Middle Name": "Compared with the first name",
            "ID type": "Seafarer's Book",
        })
        await expect(
            canvas.getByText("The current simulation gives the same decision as the recorded one.")
        ).toBeVisible()
        await edit(canvasElement)
        expect(within(draft(canvasElement)).queryByRole("group", {name: "Middle Name"})).toBeNull()
    },
}

export const EnrollmentDecidedSinceByAnOfficer: Story = {
    args: {source: "carol-approved"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(rule(canvasElement)).toBe("2"))
        await expect(canvasElement).toHaveTextContent(
            "Rule 2 of version 1 · Send to a person. Its status is now Approved."
        )
    },
}

export const AnEnrollmentWhoseDecisionCantBeRead: Story = {
    args: {source: "carol-unreadable"},
    play: async ({canvasElement}) => {
        const canvas = canvasOf(canvasElement)
        await waitFor(() => expect(rule(canvasElement)).toBe("3"))
        await expect(
            canvas.getByText(
                "The recorded decision of this enrollment can't be read, so the simulation uses the sample enrollment."
            )
        ).toBeVisible()
        expect(summary(canvasElement)).toMatchObject({"Identity check": "Verified by ID scan"})
        expect(canvasElement.textContent).not.toMatch(/Recorded decision|recorded one/)
        await expect(
            canvas.getByRole("button", {name: "Back to the enrollment of Carol Sample"})
        ).toBeVisible()
    },
}

export const ALongName: Story = {
    args: {source: "long-name"},
    globals: {viewport: {value: "iphone5", isRotated: false}},
    play: async ({canvasElement}) => {
        const back = await canvasOf(canvasElement).findByRole("button", {
            name: /^Back to the enrollment of Maximiliana/,
        })
        const panel = canvasOf(canvasElement).getByTestId("panel")
        expect(panel.scrollWidth).toBeLessThanOrEqual(panel.clientWidth)
        expect(back.getBoundingClientRect().right).toBeLessThanOrEqual(
            panel.getBoundingClientRect().right + 1
        )
    },
}

export const TheEditorInAPhoneWidthPanel: Story = {
    args: {source: "carol"},
    // On a phone, beside the portal's collapsed side menu, the panel is about 200 px wide.
    decorators: [
        (Story) => (
            <div style={{maxWidth: "200px"}}>
                <Story />
            </div>
        ),
    ],
    play: async ({canvasElement}) => {
        await waitFor(() => expect(rule(canvasElement)).toBe("2"))
        await edit(canvasElement)
        await choose(canvasElement, "Identity check", "Verified by ID scan")
        const editor = canvasOf(canvasElement).getByRole("group", {
            name: "Trying other details — not in the simulation yet",
        })
        const bounds = editor.getBoundingClientRect()
        // Every button stays inside the editor, its label wrapping between words only.
        for (const button of Array.from(editor.querySelectorAll("button"))) {
            const rect = button.getBoundingClientRect()
            expect(rect.left).toBeGreaterThanOrEqual(bounds.left - 0.5)
            expect(rect.right).toBeLessThanOrEqual(bounds.right + 0.5)
            const range = document.createRange()
            range.selectNodeContents(button)
            const lines = new Set(
                Array.from(range.getClientRects()).map((line) => Math.round(line.top))
            )
            expect(lines.size).toBeLessThanOrEqual(
                (button.textContent ?? "").trim().split(/\s+/).length
            )
        }
    },
}

export const Association: Story = {
    args: {matrix: "association"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(rule(canvasElement)).toBe("3"))
        expect(summary(canvasElement).Embassy).toBeUndefined()
        await edit(canvasElement)
        await choose(canvasElement, "Date Of Birth", "Different")
        await userEvent.click(canvasOf(canvasElement).getByRole("button", {name: "Apply example"}))
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                /^In the simulation, no rule fits.*Send to a person/
            )
        )
    },
}

export const RulesThatCannotBeSaved: Story = {
    args: {matrix: "invalid"},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                "Fix these rules to run the simulation:" +
                    "Rule 2: Enrollments whose identity was typed by hand can't be approved automatically."
            )
        )
        expect(rule(canvasElement)).toBe("undefined")
    },
}

export const TryingFails: Story = {
    args: {evaluates: "error"},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent("The simulation could not be run.")
        )
        expect(rule(canvasElement)).toBe("undefined")
    },
}

export const StillDeciding: Story = {
    args: {evaluates: "pending", source: "carol"},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(outcome(canvasElement)).toHaveTextContent(
                "Working out which rule decides the simulation…"
            )
        )
        expect(rule(canvasElement)).toBe("undefined")
        expect(canvasElement.textContent).not.toMatch(/recorded one/)
        expect(canvasOf(canvasElement).queryByRole("button", {name: /^Go to/})).toBeNull()
    },
}
