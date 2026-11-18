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
import {EStoryLocale} from "../../../../ui-essentials/.storybook/globals"
import {atViewport} from "../../../../test-support/storybook/viewport"
import {NARROW_CARD} from "./ApprovalMatrix"
import {ApprovalMatrixTest, decidingRule, useExampleEvaluation} from "./ApprovalMatrixTest"
import {EFieldMatch, EIdentityMethod, ITestEnrollment, humanizeField} from "./approvalMatrix"
import {EMatrixEntry, recordedDecision, sampleEnrollment} from "./approvalExample"
import {IApplication, applicantName} from "./approvalReview"
import {Card} from "./approvalStyles"
import {editorContent, expectEditorButtons} from "./__stories__/editorButtons"
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
        await expect(
            within(draft(canvasElement)).getByRole("button", {name: "Apply"})
        ).toBeDisabled()
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

        await userEvent.click(within(draft(canvasElement)).getByRole("button", {name: "Apply"}))
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
        await expect(
            within(draft(canvasElement)).getByRole("button", {name: "Apply"})
        ).toBeEnabled()
        await userEvent.click(canvas.getByRole("button", {name: "Fill in the sample"}))
        await expect(
            within(draft(canvasElement)).getByRole("button", {name: "Apply"})
        ).toBeDisabled()
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
        await userEvent.click(within(draft(canvasElement)).getByRole("button", {name: "Apply"}))
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
        await userEvent.click(within(draft(canvasElement)).getByRole("button", {name: "Apply"}))
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
        await userEvent.click(within(draft(canvasElement)).getByRole("button", {name: "Apply"}))
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

export const FillingInWithTheKeyboard: Story = {
    args: {source: "carol"},
    play: async ({canvasElement}) => {
        const canvas = canvasOf(canvasElement)
        await waitFor(() => expect(rule(canvasElement)).toBe("2"))
        await edit(canvasElement)
        const editor = within(draft(canvasElement))
        const fill = editor.getByRole("button", {name: "Fill in the recorded details"})
        const apply = editor.getByRole("button", {name: "Apply"})
        const identity = () =>
            editor
                .getByRole("group", {name: "Identity check"})
                .querySelector("[aria-pressed='true']")?.textContent
        // With the recorded details in the editor, Enter on it does nothing and keeps focus.
        fill.focus()
        await userEvent.keyboard("{Enter}")
        await expect(fill).toHaveFocus()
        await expect(fill).toHaveAttribute("aria-disabled", "true")
        expect(identity()).toBe("Typed by hand")
        // After an edit, Enter puts the recorded details back in the editor only.
        await choose(canvasElement, "Identity check", "Verified by ID scan")
        await expect(apply).toBeEnabled()
        fill.focus()
        await userEvent.keyboard("{Enter}")
        expect(identity()).toBe("Typed by hand")
        await expect(apply).toBeDisabled()
        await expect(fill).toHaveAttribute("aria-disabled", "true")
        // The simulation never changed: still the recorded details, still rule 2.
        expect(summary(canvasElement)["Identity check"]).toBe("Typed by hand")
        expect(rule(canvasElement)).toBe("2")
        expect(
            canvas.getByRole("heading", {level: 4, name: "Recorded details · in the simulation"})
        ).toBeVisible()
    },
}

/** The editor as on the portal's page: its card's width and the editor's content width there. */
interface IEditorLayout {
    viewport: number
    card: number
    content: number
    fontSize: string
}
// A 375 px phone and the smallest, 320 px, beside the collapsed side menu, and a 1280 px desktop.
const PHONE: IEditorLayout = {viewport: 375, card: 215, content: 181, fontSize: "14px"}
const SMALLEST_PHONE: IEditorLayout = {viewport: 320, card: 192, content: 174, fontSize: "14px"}
const DESKTOP: IEditorLayout = {viewport: 1280, card: 360, content: 274, fontSize: "16px"}

/**
 * The editor's buttons: the fill-in button on its own row, then Cancel at the left and Apply at
 * the right of one row, inside the editor, no word broken, in that keyboard order.
 */
const editorButtons = (layout: IEditorLayout, locale = EStoryLocale.ENGLISH): Story => {
    const viewport = atViewport(layout.viewport)
    return {
        ...viewport,
        args: {source: "carol"},
        globals: {...viewport.globals, locale},
        render: ({matrix, source, onBack}) => (
            <MatrixScreen canEdit={false}>
                <Card
                    aria-labelledby="approval-matrix-example"
                    sx={{...NARROW_CARD, width: `${layout.card}px`, boxSizing: "border-box"}}
                >
                    <Panel matrix={matrix} application={SOURCES[source]} onBack={onBack} />
                </Card>
            </MatrixScreen>
        ),
        play: async ({canvasElement}) => {
            await waitFor(() => expect(rule(canvasElement)).toBe("2"))
            await userEvent.click(
                await canvasOf(canvasElement).findByRole("button", {
                    name: String(EDIT_DETAILS[locale]),
                })
            )
            const editor = canvasElement.querySelector(
                "[aria-labelledby='approval-matrix-draft']"
            ) as HTMLElement
            // The editor is as wide as on the portal's page.
            expect(Math.round(editorContent(editor).width)).toBe(layout.content)

            const [fill, cancel, apply] = Array.from(editor.querySelectorAll("button")).slice(-3)
            // With nothing edited yet, filling in does nothing, and says so; it stays focusable,
            // and the keyboard skips the disabled Apply.
            await expect(fill).toHaveAttribute("aria-disabled", "true")
            await expect(apply).toBeDisabled()
            fill.focus()
            await userEvent.tab()
            await expect(cancel).toHaveFocus()
            await userEvent.tab()
            await expect(apply).not.toHaveFocus()

            // An edit: the first choice of the identity check that isn't the current one.
            await userEvent.click(
                editor.querySelector("[role='group'] button[aria-pressed='false']") as HTMLElement
            )
            await expect(fill).toHaveAttribute("aria-disabled", "false")
            await expect(apply).toBeEnabled()
            expect(apply).toHaveTextContent(String(APPLY[locale]))

            await expectEditorButtons(editor, layout.fontSize)
        },
    }
}

const EDIT_DETAILS: Partial<Record<EStoryLocale, string>> = {
    [EStoryLocale.ENGLISH]: "Try other details",
    [EStoryLocale.DUTCH]: "Andere gegevens proberen",
    [EStoryLocale.FRENCH]: "Essayer d'autres données",
    [EStoryLocale.CATALAN]: "Prova altres dades",
    [EStoryLocale.TAGALOG]: "Subukan ang ibang detalye",
}
const APPLY: Partial<Record<EStoryLocale, string>> = {
    [EStoryLocale.ENGLISH]: "Apply",
    [EStoryLocale.DUTCH]: "Toepassen",
    [EStoryLocale.FRENCH]: "Appliquer",
    [EStoryLocale.CATALAN]: "Aplica",
    [EStoryLocale.TAGALOG]: "Ilapat",
}
export const TheEditorsButtonsOnAPhone = editorButtons(PHONE)
export const TheEditorsButtonsOnADesktop = editorButtons(DESKTOP)
// The longest Cancel and Apply: Dutch, French and Catalan; Tagalog's hyphenated I-cancel.
export const TheEditorsButtonsInDutchOnAPhone = editorButtons(PHONE, EStoryLocale.DUTCH)
export const TheEditorsButtonsInDutchOnTheSmallestPhone = editorButtons(
    SMALLEST_PHONE,
    EStoryLocale.DUTCH
)
export const TheEditorsButtonsInDutchOnADesktop = editorButtons(DESKTOP, EStoryLocale.DUTCH)
export const TheEditorsButtonsInFrenchOnAPhone = editorButtons(PHONE, EStoryLocale.FRENCH)
export const TheEditorsButtonsInCatalanOnAPhone = editorButtons(PHONE, EStoryLocale.CATALAN)
export const TheEditorsButtonsInTagalogOnTheSmallestPhone = editorButtons(
    SMALLEST_PHONE,
    EStoryLocale.TAGALOG
)

export const Association: Story = {
    args: {matrix: "association"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(rule(canvasElement)).toBe("3"))
        expect(summary(canvasElement).Embassy).toBeUndefined()
        await edit(canvasElement)
        await choose(canvasElement, "Date Of Birth", "Different")
        await userEvent.click(within(draft(canvasElement)).getByRole("button", {name: "Apply"}))
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
