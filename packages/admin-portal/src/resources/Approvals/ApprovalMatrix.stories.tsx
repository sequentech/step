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
import {EIdentityMethod, EMatrixReason, ITestEnrollment} from "./approvalMatrix"
import {applicationRecord, applications} from "./__stories__/ApprovalsFixture"
import {
    MatrixScreen,
    comelecMatrix,
    matrixCalls,
    setUpMatrix,
    unexpectedCalls,
    type MatrixServices,
} from "./__stories__/ApprovalMatrixFixture"

/** The enrollments the matrix can be opened from. */
const SOURCES = {
    alice: () => applicationRecord(),
    carol: () => applications()[2],
    dan: () => applications()[3],
    unreadable: () =>
        applicationRecord({
            annotations: {
                decision: {
                    matrix_version: 1,
                    rule: 5,
                    decision: "PENDING",
                    inputs: {identity: "VERIFIED", voter_found: "yes"},
                    candidates: 1,
                },
            },
        }),
    long: () =>
        applicationRecord({
            applicant_data: {
                firstName: "Maximiliana-Bartholomea-Wilhelmina",
                lastName: "Featherstonehaugh-Montgomery-Abernathy-Delacroix",
            },
        }),
}

interface Scenario extends MatrixServices {
    /** The enrollment the matrix was opened from; none for the toolbar. */
    source?: keyof typeof SOURCES
    goBack: Mock<() => void>
    backToSource: Mock<() => void>
}

const meta = {
    title: "Admin/Approvals/ApprovalMatrix",
    component: ApprovalMatrix,
    args: {
        saved: "built-in",
        canEdit: true,
        reads: "matrix",
        saves: "saved",
        goBack: fn(),
        backToSource: fn(),
    },
    argTypes: {
        saved: {control: "inline-radio", options: ["built-in", "association"]},
        reads: {control: "inline-radio", options: ["matrix", "loading", "error"]},
        saves: {control: "inline-radio", options: ["saved", "error"]},
    },
    beforeEach: ({args}) => setUpMatrix(args),
    render: ({canEdit, goBack, source, backToSource}) => (
        <MatrixScreen canEdit={canEdit}>
            <div data-testid="screen">
                <ApprovalMatrix
                    electionEventId={EVENT_ID}
                    goBack={goBack}
                    source={
                        source ? {application: SOURCES[source](), onBack: backToSource} : undefined
                    }
                />
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
    within(canvasElement).getByRole("status", {name: "Current simulation"})

/** The positions of the highlighted rule cards, `0` for the last rule. */
const highlighted = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll("li[data-highlighted='true']")).map((card) =>
        card.id.replace("approval-matrix-rule-", "")
    )

const draft = (canvasElement: HTMLElement) =>
    within(
        within(canvasElement).getByRole("group", {
            name: /^(Editing the example|Trying other details) — not/,
        })
    )

/** The labels of the applied example's summary. */
const summaryTerms = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll("dt")).map((term) => term.textContent)

/** Whether the button's centre is on screen and not covered, so it can be clicked there. */
const reachable = (button: HTMLElement) => {
    const box = button.getBoundingClientRect()
    const hit = document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2)
    return box.bottom <= window.innerHeight && button.contains(hit)
}

/** Chooses an option of the example being edited. */
const choose = (canvasElement: HTMLElement, label: string, option: string) =>
    userEvent.click(
        within(draft(canvasElement).getByRole("group", {name: label})).getByRole("button", {
            name: option,
        })
    )

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
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent("In the simulation, rule 3 decides")
        )
        await expect(await within(cards[2]).findByText("Decides in the simulation")).toBeVisible()
        await expect(cards[2]).toHaveAttribute("aria-current", "true")
        expect(highlighted(canvasElement)).toEqual(["3"])
        await expect(canvas.getByText("Sample enrollment")).toBeVisible()
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
        expect(
            canvas.queryByRole("button", {name: /^(Edit|Move|Delete) rule|^Edit the last rule/})
        ).toBeNull()
        expect(canvas.queryByRole("button", {name: "Add rule"})).toBeNull()
        // An example can still be tried.
        await userEvent.click(canvas.getByRole("button", {name: "Edit example"}))
        await choose(canvasElement, "Identity check", "Typed by hand")
        await userEvent.click(canvas.getByRole("button", {name: "Apply example"}))
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent("In the simulation, rule 2 decides")
        )
        expect(highlighted(canvasElement)).toEqual(["2"])
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
        await waitFor(() => expect(summaryTerms(canvasElement)).toContain("Embassy"))
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
        await waitFor(() => expect(summaryTerms(canvasElement)).not.toContain("Embassy"))
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
        await waitFor(() => expect(summaryTerms(canvasElement)).toContain("Email"))
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
                "Fix these rules to run the simulation:Choose at least one detail to compare with the registry."
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
    args: {source: "alice"},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const cards = await ruleCards(canvasElement)
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent("In the simulation, rule 5 decides")
        )
        // One rule is highlighted: the one that decides the simulation.
        expect(highlighted(canvasElement)).toEqual(["5"])
        await expect(within(cards[4]).getByText("Decides in the simulation")).toBeVisible()
        await expect(canvas.getByText("Simulating the enrollment of Alice Example")).toBeVisible()
        await expect(canvasElement).toHaveTextContent("Rule 5 of version 1 · Send to a person.")
        await expect(canvas.getByText("Rules on screen")).toBeVisible()
        // The panel comes before the rules, for reading and keyboard order.
        const panel = canvas.getByRole("region", {
            name: "Simulating the enrollment of Alice Example",
        })
        expect(
            panel.compareDocumentPosition(canvas.getByRole("list", {name: "Rules"})) &
                Node.DOCUMENT_POSITION_FOLLOWING
        ).toBeTruthy()
        // Its first control is the way back: short on screen, with the name for assistive technology.
        const back = panel.querySelector("button") as HTMLElement
        expect(back).toHaveTextContent("Back to the enrollment")
        expect(back).toHaveAccessibleName("Back to the enrollment of Alice Example")
        await userEvent.click(
            canvas.getByRole("button", {name: "Back to the enrollment of Alice Example"})
        )
        expect(args.backToSource).toHaveBeenCalledTimes(1)
        await userEvent.click(canvas.getByRole("button", {name: "Approvals"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const OpenedFromAnEnrollmentWithoutAVoter: Story = {
    args: {source: "dan"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent(
                /^In the simulation, no rule fits: the last rule decides.*Reject/
            )
        )
        expect(highlighted(canvasElement)).toEqual(["otherwise"])
        await expect(
            canvas.getByText("The current simulation gives the same decision as the recorded one.")
        ).toBeVisible()
    },
}

export const TheRecordedRuleIsNeverHighlighted: Story = {
    args: {source: "carol"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["2"]))
        await userEvent.click(canvas.getByRole("button", {name: "Try other details"}))
        await choose(canvasElement, "Identity check", "Verified by ID scan")
        await userEvent.click(canvas.getByRole("button", {name: "Use in the simulation"}))
        // Carol's recorded rule 2 stays context: only the simulation's rule is highlighted.
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["3"]))
        await expect(canvasElement).toHaveTextContent("Rule 2 of version 1 · Send to a person.")
        await expect(
            canvas.getByText(
                "These aren't the recorded details, so the simulation isn't compared with the recorded decision."
            )
        ).toBeVisible()
    },
}

export const TheDraftWaitsWhileRulesChange: Story = {
    args: {source: "alice"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["5"]))
        await userEvent.click(canvas.getByRole("button", {name: "Try other details"}))
        await choose(canvasElement, "Identity check", "Typed by hand")

        await userEvent.click(canvas.getByRole("button", {name: "Move rule 5 up"}))
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["4"]))
        await userEvent.click(canvas.getByRole("button", {name: "Delete rule 1"}))
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["3"]))
        await userEvent.click(compared(canvasElement).getByRole("button", {name: "Middle Name"}))
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent("In the simulation, rule 3 decides")
        )
        await userEvent.click(canvas.getByRole("button", {name: "Edit rule 2"}))
        const editor = await dialog("Edit rule 2")
        await userEvent.click(editor.getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())

        // The draft is still open and was never decided.
        await expect(
            draft(canvasElement)
                .getByRole("group", {name: "Identity check"})
                .querySelector("[aria-pressed='true']")
        ).toHaveTextContent("Typed by hand")
        expect(draft(canvasElement).queryByRole("group", {name: "Middle Name"})).toBeNull()
        expect(
            matrixCalls("EvaluateApprovalMatrix").every(
                ({variables}) =>
                    (variables.enrollment as ITestEnrollment).identity === EIdentityMethod.VERIFIED
            )
        ).toBe(true)
        await expect(canvas.getByText("Rules on screen, including unsaved changes")).toBeVisible()

        await userEvent.click(canvas.getByRole("button", {name: "Use in the simulation"}))
        // The rule that was rule 2 is now first.
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["1"]))
    },
}

/** Whether each highlighted card is the rule "All details match", the one that decides the sample. */
const highlightsAllMatch = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll("li[data-highlighted='true']")).every((card) =>
        /When\s*All details match/.test(card.textContent ?? "")
    )

export const MovingTheDecidingRule: Story = {
    // Later requests are answered first, so answers for earlier rule orders arrive late.
    args: {evaluates: "reversed"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["3"]), {timeout: 3000})
        // Every state the page shows on the way: no rule, or the rule that decides the sample.
        const seen: Array<{rules: string[]; allMatch: boolean}> = []
        let observed = 0
        const record = () =>
            seen.push({
                rules: highlighted(canvasElement),
                allMatch: highlightsAllMatch(canvasElement),
            })
        const observer = new MutationObserver(() => {
            observed++
            record()
        })
        observer.observe(canvasElement, {subtree: true, attributes: true, childList: true})
        // Move it down, then delete the first rule before the first answer arrives.
        await userEvent.click(canvas.getByRole("button", {name: "Move rule 3 down"}))
        record()
        await userEvent.click(canvas.getByRole("button", {name: "Delete rule 1"}))
        record()
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["3"]), {timeout: 3000})
        // Let the late answer for the moved order arrive: it must not move the highlight.
        await new Promise((resolve) => setTimeout(resolve, 1500))
        record()
        observer.disconnect()
        expect(highlighted(canvasElement)).toEqual(["3"])
        // The observer saw the page change (rule order, highlight, result), not only the checks above.
        expect(observed).toBeGreaterThan(0)
        for (const {rules, allMatch} of seen) {
            expect(rules.length).toBeLessThanOrEqual(1)
            expect(allMatch).toBe(true)
        }
    },
}

export const OpenedFromAnEnrollmentWhoseDecisionCantBeRead: Story = {
    args: {source: "unreadable"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await expect(
            await canvas.findByText(
                "The recorded decision of this enrollment can't be read, so the simulation uses the sample enrollment."
            )
        ).toBeVisible()
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["3"]))
        expect(canvasElement.textContent).not.toMatch(/Recorded decision|recorded one/)
        await expect(
            canvas.getByRole("button", {name: "Back to the enrollment of Alice Example"})
        ).toBeVisible()
    },
}

export const TheDraftWaitsThroughRuleEditsSavesAndDiscards: Story = {
    args: {source: "alice"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["5"]))
        await userEvent.click(canvas.getByRole("button", {name: "Try other details"}))
        await choose(canvasElement, "Identity check", "Typed by hand")
        const stillDrafted = async () => {
            await expect(
                draft(canvasElement)
                    .getByRole("group", {name: "Identity check"})
                    .querySelector("[aria-pressed='true']")
            ).toHaveTextContent("Typed by hand")
            expect(
                matrixCalls("EvaluateApprovalMatrix").every(
                    ({variables}) =>
                        (variables.enrollment as ITestEnrollment).identity ===
                        EIdentityMethod.VERIFIED
                )
            ).toBe(true)
        }

        // Applying a change in the rule editor applies the rule, not the example.
        await userEvent.click(canvas.getByRole("button", {name: "Edit rule 5"}))
        const editor = await dialog("Edit rule 5")
        await userEvent.click(editor.getByRole("radio", {name: /^Reject/}))
        await userEvent.click(editor.getByRole("button", {name: "Apply"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent(
                /^In the simulation, rule 5 decides.*Reject/
            )
        )
        await stillDrafted()

        const bar = within(await canvas.findByRole("region", {name: "Unsaved changes"}))
        await userEvent.click(bar.getByRole("button", {name: "Discard changes"}))
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent(
                /^In the simulation, rule 5 decides.*Send to a person/
            )
        )
        await stillDrafted()

        await userEvent.click(canvas.getByRole("button", {name: "Delete rule 7"}))
        const saveBar = within(await canvas.findByRole("region", {name: "Unsaved changes"}))
        await userEvent.click(saveBar.getByRole("button", {name: "Save as version 2"}))
        await userEvent.click(
            (await dialog("Save as version 2?")).getByRole("button", {name: "Save version 2"})
        )
        await expect(await canvas.findByText("Version 2")).toBeVisible()
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["5"]))
        await stillDrafted()

        await userEvent.click(canvas.getByRole("button", {name: "Use in the simulation"}))
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["2"]))
    },
}

export const AShortMatrixKeepsTheWholeCard: Story = {
    args: {saved: "association"},
    globals: {viewport: {value: "desktop-1280", isRotated: false}},
    parameters: {
        viewport: {
            options: {
                "desktop-1280": {name: "1280 × 800", styles: {width: "1280px", height: "800px"}},
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        const card = canvas.getByRole("region", {name: "Try an example"})
        const cap = () => parseFloat(getComputedStyle(card).maxHeight)
        // Empty window space below a short page doesn't shrink the card: it needs no scrolling.
        await waitFor(() => expect(card.scrollHeight).toBeLessThanOrEqual(card.clientHeight + 1))
        const before = cap()
        expect(before).toBeGreaterThan(card.offsetHeight)

        await userEvent.click(canvas.getByRole("button", {name: "Delete rule 3"}))
        const bar = await canvas.findByRole("region", {name: "Unsaved changes"})
        // With the save bar, the card's room shrinks by at least the bar's height.
        await waitFor(() => expect(cap()).toBeLessThanOrEqual(before - bar.offsetHeight))
        await userEvent.click(within(bar).getByRole("button", {name: "Discard changes"}))
        await waitFor(() => expect(unsavedBar(canvasElement)).toBeNull())
        // Without it, the card gets its room back.
        await waitFor(() => expect(cap()).toBe(before))
        expect(card.scrollHeight).toBeLessThanOrEqual(card.clientHeight + 1)
    },
}

export const ASaveBarThatAppearsMidPage: Story = {
    args: {source: "alice"},
    globals: {viewport: {value: "desktop-1280", isRotated: false}},
    parameters: {
        viewport: {
            options: {
                "desktop-1280": {name: "1280 × 800", styles: {width: "1280px", height: "800px"}},
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Try other details"}))
        // The save bar appears while the page is scrolled half way: it is drawn stuck to the
        // bottom of the window, above the end of the rules.
        window.scrollTo(0, (document.documentElement.scrollHeight - window.innerHeight) / 2)
        await userEvent.click(canvas.getByRole("button", {name: "Delete rule 4"}))
        await canvas.findByRole("region", {name: "Unsaved changes"})
        await new Promise((resolve) => setTimeout(resolve, 300))
        // At the end of the page the card still starts inside the window, with its editor's heading.
        window.scrollTo(0, document.documentElement.scrollHeight)
        const card = canvas.getByRole("region", {
            name: "Simulating the enrollment of Alice Example",
        })
        await waitFor(() => {
            const top = card.getBoundingClientRect().top
            expect(top).toBeGreaterThanOrEqual(-1)
            const heading = document.getElementById("approval-matrix-draft") as HTMLElement
            expect(heading.getBoundingClientRect().top).toBeGreaterThanOrEqual(top)
        })
    },
}

export const StillDeciding: Story = {
    args: {evaluates: "pending", source: "alice"},
    play: async ({canvasElement}) => {
        await ruleCards(canvasElement)
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent(
                "Working out which rule decides the simulation…"
            )
        )
        expect(highlighted(canvasElement)).toEqual([])
    },
}

export const TryingFails: Story = {
    args: {evaluates: "error"},
    play: async ({canvasElement}) => {
        await ruleCards(canvasElement)
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent("The simulation could not be run.")
        )
        expect(highlighted(canvasElement)).toEqual([])
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
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["3"]))
    },
}

export const NarrowScreenFromAnEnrollmentWithALongName: Story = {
    args: {source: "long"},
    globals: {viewport: {value: "iphone5", isRotated: false}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await ruleCards(canvasElement)
        const back = canvas.getByRole("button", {name: /^Back to the enrollment of Maximiliana/})
        const screen = canvas.getByTestId("screen")
        expect(screen.scrollWidth).toBeLessThanOrEqual(screen.clientWidth)
        // The example comes first on screen and in keyboard order: after "Approvals", its way back.
        expect(back.getBoundingClientRect().top).toBeLessThan(
            canvas.getByRole("list", {name: "Rules"}).getBoundingClientRect().top
        )
        canvas.getByRole("button", {name: "Approvals"}).focus()
        await userEvent.tab()
        await expect(back).toHaveFocus()
        await userEvent.click(canvas.getByRole("button", {name: "Try other details"}))
        expect(screen.scrollWidth).toBeLessThanOrEqual(screen.clientWidth)
        await choose(canvasElement, "Identity check", "Typed by hand")
        const apply = canvas.getByRole("button", {name: "Use in the simulation"})
        await expect(apply).toBeEnabled()
        apply.focus()
        await waitFor(() => expect(reachable(apply)).toBe(true))
        const box = apply.getBoundingClientRect()
        expect(
            apply.contains(document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2))
        ).toBe(true)
    },
}

export const EditingWithUnsavedRulesAt1280: Story = {
    args: {source: "alice"},
    globals: {viewport: {value: "desktop-1280", isRotated: false}},
    parameters: {
        viewport: {
            options: {
                "desktop-1280": {
                    name: "1280 × 800",
                    styles: {width: "1280px", height: "800px"},
                },
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(window.innerWidth).toBe(1280)
        await ruleCards(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Delete rule 7"}))
        const bar = await canvas.findByRole("region", {name: "Unsaved changes"})
        // The editor makes room for the save bar as drawn, not for an assumed height.
        await waitFor(() =>
            expect(
                getComputedStyle(canvas.getByTestId("screen").firstElementChild as HTMLElement)
                    .getPropertyValue("--approval-matrix-save-bar")
                    .trim()
            ).toBe(`${bar.offsetHeight + 32}px`)
        )
        await userEvent.click(canvas.getByRole("button", {name: "Try other details"}))
        await choose(canvasElement, "Last Name", "Different")
        await expect(canvas.getByRole("button", {name: "Use in the simulation"})).toBeEnabled()
        // Moving to the editor's buttons with the keyboard brings each into view, clear of the
        // sticky save bar, even with the page scrolled to the top.
        window.scrollTo(0, 0)
        for (const name of ["Fill in the recorded details", "Cancel", "Use in the simulation"]) {
            const button = canvas.getByRole("button", {name})
            button.focus()
            await waitFor(() => expect(reachable(button)).toBe(true))
        }
        // Moving back up through the editor never leaves a control under its sticky heading.
        const headingBox = () =>
            (
                document.getElementById("approval-matrix-draft")?.parentElement as HTMLElement
            ).getBoundingClientRect()
        const controls = Array.from(
            draft(canvasElement)
                .getByRole("group", {name: "Last Name"})
                .closest("[role='group'][aria-labelledby]")
                ?.querySelectorAll<HTMLElement>("button") ?? []
        ).reverse()
        expect(controls.length).toBeGreaterThan(8)
        for (const control of controls) {
            control.focus()
            await waitFor(() => {
                expect(control.getBoundingClientRect().top).toBeGreaterThanOrEqual(
                    headingBox().bottom - 1
                )
                expect(reachable(control)).toBe(true)
            })
        }

        await userEvent.click(canvas.getByRole("button", {name: "Use in the simulation"}))
        await waitFor(() =>
            expect(example(canvasElement)).toHaveTextContent(
                /^In the simulation, no rule fits: the last rule decides/
            )
        )
        // The result of the applied example is in view where the editor was.
        await waitFor(() => {
            const result = example(canvasElement).getBoundingClientRect()
            const card = canvas
                .getByRole("region", {name: "Simulating the enrollment of Alice Example"})
                .getBoundingClientRect()
            expect(result.top).toBeGreaterThanOrEqual(card.top)
            expect(result.bottom).toBeLessThanOrEqual(Math.min(card.bottom, window.innerHeight))
        })
        await expect(canvas.getByRole("button", {name: "Try other details"})).toHaveFocus()
    },
}
