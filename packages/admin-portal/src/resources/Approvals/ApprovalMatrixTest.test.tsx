/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import i18next from "i18next"
import {I18nextProvider, initReactI18next} from "react-i18next"
import en from "@/translations/en"
import {IApplicationsStatus} from "@/types/applications"
import {
    ApprovalMatrixTest,
    ApprovalMatrixTestProps,
    EExampleResult,
    IExampleEvaluation,
} from "./ApprovalMatrixTest"
import {EIdentityMethod, ITestEnrollment, humanizeField} from "./approvalMatrix"
import {EMatrixEntry, recordedDecision, sampleEnrollment} from "./approvalExample"
import {IApplication} from "./approvalReview"

const {ACCEPTED, PENDING} = IApplicationsStatus
const COMPARED = ["firstName", "middleName", "lastName", "dateOfBirth", "embassy"]
const VALID_IDS = ["philippinePassport", "seamanBook"]

const i18n = i18next.createInstance()

beforeAll(async () => {
    await i18n.use(initReactI18next).init({
        lng: "en",
        defaultNS: "translations",
        resources: {en},
        interpolation: {escapeValue: false},
    })
    // jsdom lays nothing out, so it can't scroll either; each test spies on its own scrolling.
    Element.prototype.scrollIntoView = () => undefined
    globalThis.ResizeObserver = class {
        observe() {}
        disconnect() {}
        unobserve() {}
    } as unknown as typeof ResizeObserver
})

/** Carol typed her details by hand; her decision recorded every compared detail but the middle name. */
const carol = (decision: Record<string, unknown> = {}, name = "Carol"): IApplication => ({
    status: PENDING,
    applicant_data: {firstName: name, lastName: name ? "Sample" : ""},
    annotations: {
        decision: {
            matrix_version: 1,
            matrix_source: "BUILT_IN",
            rule: 2,
            conditions: {identity: "MANUAL_ENTRY"},
            decision: PENDING,
            reason: "IDENTITY_NOT_VERIFIED",
            inputs: {
                identity: "MANUAL_ENTRY",
                voter_found: true,
                already_enrolled: false,
                valid_id: null,
                fields: {
                    firstName: "MATCHES",
                    lastName: "DIFFERS",
                    dateOfBirth: "MATCHES",
                    embassy: "MATCHES",
                },
                differing: 1,
            },
            candidates: 1,
            accepted_candidates: 0,
            ...decision,
        },
    },
})

const decided = (rule: number | null, decision = PENDING): IExampleEvaluation => ({
    state: EExampleResult.DECIDED,
    result: {rule, decision, reason: null, invariant: null, errors: []},
})

interface Setup {
    application?: IApplication
    applied?: ITestEnrollment
    evaluation?: IExampleEvaluation
}

function setUp({application, applied, evaluation = decided(3, ACCEPTED)}: Setup = {}) {
    const recorded = application ? recordedDecision(application) : null
    const entry = recorded?.enrollment ? EMatrixEntry.ENROLLMENT : EMatrixEntry.TOOLBAR
    const props: ApprovalMatrixTestProps = {
        applied: applied ?? recorded?.enrollment ?? sampleEnrollment(),
        onApply: jest.fn(),
        entry,
        joint: false,
        source: application && {
            name: [application.applicant_data as Record<string, string>]
                .map(({firstName, lastName}) => [firstName, lastName].filter(Boolean).join(" "))
                .join(""),
            status: application.status,
            recorded,
            onBack: jest.fn(),
        },
        comparedFields: COMPARED,
        validIds: VALID_IDS,
        fieldLabel: humanizeField,
        evaluation,
        unsavedRules: false,
        onGoToRule: jest.fn(),
    }
    // The matrix screen's card is labelled by the panel's title.
    render(
        <I18nextProvider i18n={i18n}>
            <section aria-labelledby="approval-matrix-example">
                <ApprovalMatrixTest {...props} />
            </section>
        </I18nextProvider>
    )
    return props
}

const heading = (level: number, name: string | RegExp) => screen.getByRole("heading", {level, name})
const summary = () =>
    Object.fromEntries(
        Array.from(document.querySelectorAll("dt")).map((term) => [
            term.textContent,
            term.nextElementSibling?.textContent,
        ])
    )
const draft = (name: string) => within(screen.getByRole("group", {name}))
const choose = (group: ReturnType<typeof draft>, label: string, option: string) =>
    fireEvent.click(
        within(group.getByRole("group", {name: label})).getByRole("button", {name: option})
    )

describe("opened from the toolbar", () => {
    it("tries the sample enrollment", () => {
        setUp()
        expect(screen.getByRole("region", {name: "Try an example"})).toBeInTheDocument()
        expect(heading(3, "Try an example")).toBeInTheDocument()
        expect(heading(4, "Sample enrollment")).toBeInTheDocument()
        expect(
            screen.getByText(
                "A verified voter in the registry, not enrolled yet, whose compared details are all the same."
            )
        ).toBeInTheDocument()
        expect(
            screen.getByText("Trying an example changes no enrollment and saves nothing.")
        ).toBeInTheDocument()
        expect(screen.queryByRole("button", {name: /^Back to/})).toBeNull()
        expect(screen.queryByText(/Recorded decision/)).toBeNull()
        expect(summary()).toMatchObject({
            "Identity check": "Verified by ID scan",
            "Embassy": "Same",
        })
    })

    it("calls an example that isn't the sample the administrator's own", () => {
        setUp({applied: {...sampleEnrollment(), identity: EIdentityMethod.MANUAL_ENTRY}})
        expect(heading(3, "Try an example")).toBeInTheDocument()
        expect(heading(4, "Your example")).toBeInTheDocument()
        expect(screen.getByText("Started from the sample.")).toBeInTheDocument()
        expect(screen.queryByText(/whose compared details are all the same/)).toBeNull()
    })

    it("edits the example and applies it only on Apply", async () => {
        const scrolled = jest.spyOn(Element.prototype, "scrollIntoView")
        const props = setUp()
        fireEvent.click(screen.getByRole("button", {name: "Edit example"}))
        const editor = draft("Editing the example — not applied yet")
        expect(editor.getByRole("button", {name: "Apply"})).toBeDisabled()
        choose(editor, "Identity check", "Typed by hand")
        expect(props.onApply).not.toHaveBeenCalled()
        expect(summary()["Identity check"]).toBe("Verified by ID scan")
        fireEvent.click(editor.getByRole("button", {name: "Fill in the sample"}))
        expect(editor.getByRole("button", {name: "Apply"})).toBeDisabled()
        choose(editor, "Voter in the registry", "No")
        fireEvent.click(editor.getByRole("button", {name: "Apply"}))
        expect(props.onApply).toHaveBeenCalledWith({...sampleEnrollment(), voter_found: false})
        expect(screen.queryByRole("group", {name: /^Editing/})).toBeNull()
        // The details in use come back into view, and focus returns to the button that opened
        // the editor.
        await waitFor(() =>
            expect(screen.getByRole("button", {name: "Edit example"})).toHaveFocus()
        )
        expect(scrolled).toHaveBeenCalledWith({block: "nearest"})
        const section = scrolled.mock.contexts.at(-1) as HTMLElement
        expect(within(section).getByRole("heading", {level: 4})).toHaveTextContent(
            "Sample enrollment"
        )
        scrolled.mockRestore()
    })
})

describe("opened from an enrollment", () => {
    it("leads with the enrollment it simulates, its way back and its recorded decision", () => {
        const props = setUp({application: carol(), evaluation: decided(2)})
        // The card is about the enrollment, never "Try an example".
        expect(
            screen.getByRole("region", {name: "Simulating the enrollment of Carol Sample"})
        ).toBeInTheDocument()
        expect(heading(3, "Simulating the enrollment of Carol Sample")).toBeInTheDocument()
        const back = screen.getByRole("button", {name: "Back to the enrollment of Carol Sample"})
        expect(back).toHaveTextContent("Back to the enrollment")
        fireEvent.click(back)
        expect(props.source?.onBack).toHaveBeenCalledTimes(1)
        expect(heading(4, "Recorded decision")).toBeInTheDocument()
        expect(
            screen.getByText("Rule 2 of version 1 · Send to a person.", {exact: false})
        ).toBeInTheDocument()
        expect(
            screen.getByText("The simulation changes nothing in this enrollment and saves nothing.")
        ).toBeInTheDocument()
        expect(heading(4, "Recorded details · in the simulation")).toBeInTheDocument()
        expect(screen.queryByText("Try an example")).toBeNull()
        // Carol's decision didn't record the middle name: never assumed to be the same.
        expect(summary()).toMatchObject({"Middle Name": "Not recorded", "Last Name": "Different"})
        expect(
            screen.getByText(
                "A compared detail wasn't recorded, so the simulation isn't compared with the recorded decision."
            )
        ).toBeInTheDocument()
    })

    it("tries other details without changing the simulation until they are applied", async () => {
        const props = setUp({application: carol(), evaluation: decided(2)})
        fireEvent.click(screen.getByRole("button", {name: "Try other details"}))
        const editor = draft("Trying other details — not in the simulation yet")
        expect(
            editor.getByText("The highlighted rule still decides the simulation.")
        ).toBeInTheDocument()
        // The editor is described by what applying means.
        expect(
            screen.getByRole("group", {name: "Trying other details — not in the simulation yet"})
        ).toHaveAccessibleDescription(
            "These details decide nothing until you apply them. Until then the simulation uses the details above with the rules on screen."
        )
        const use = editor.getByRole("button", {name: "Apply"})
        expect(use).toBeDisabled()
        // Nothing to fill in yet: the button says so and changes nothing.
        const fill = editor.getByRole("button", {name: "Fill in the recorded details"})
        expect(fill).toHaveAttribute("aria-disabled", "true")
        fireEvent.click(fill)
        expect(use).toBeDisabled()
        choose(editor, "Identity check", "Verified by ID scan")
        expect(use).toBeEnabled()
        expect(props.onApply).not.toHaveBeenCalled()
        expect(heading(4, "Recorded details · in the simulation")).toBeInTheDocument()

        // Filling in the recorded details only fills the editor.
        expect(fill).toHaveAttribute("aria-disabled", "false")
        fireEvent.click(fill)
        expect(fill).toHaveAttribute("aria-disabled", "true")
        expect(use).toBeDisabled()
        expect(props.onApply).not.toHaveBeenCalled()

        fireEvent.click(editor.getByRole("button", {name: "Cancel"}))
        expect(screen.queryByRole("group", {name: /^Trying other details/})).toBeNull()
        expect(props.onApply).not.toHaveBeenCalled()
        await waitFor(() =>
            expect(screen.getByRole("button", {name: "Try other details"})).toHaveFocus()
        )

        // Editing again starts from the details in the simulation.
        fireEvent.click(screen.getByRole("button", {name: "Try other details"}))
        const again = draft("Trying other details — not in the simulation yet")
        choose(again, "Middle Name", "Same")
        fireEvent.click(again.getByRole("button", {name: "Apply"}))
        expect(props.onApply).toHaveBeenCalledTimes(1)
        expect(props.onApply).toHaveBeenCalledWith(
            expect.objectContaining({
                identity: EIdentityMethod.MANUAL_ENTRY,
                fields: expect.objectContaining({middleName: "MATCHES"}),
            })
        )
    })

    it("says when other details are in the simulation, and that the enrollment keeps its own", () => {
        const application = carol()
        const recorded = recordedDecision(application)?.enrollment as ITestEnrollment
        setUp({
            application,
            applied: {...recorded, identity: EIdentityMethod.VERIFIED},
            evaluation: decided(5),
        })
        expect(heading(3, "Simulating the enrollment of Carol Sample")).toBeInTheDocument()
        expect(heading(4, "Other details · in the simulation")).toBeInTheDocument()
        expect(screen.getByText("The enrollment keeps its recorded details.")).toBeInTheDocument()
        expect(
            screen.getByText(
                "These aren't the recorded details, so the simulation isn't compared with the recorded decision."
            )
        ).toBeInTheDocument()
    })

    it("compares the decision of complete recorded details", () => {
        const application = carol({
            inputs: {
                identity: "MANUAL_ENTRY",
                voter_found: true,
                already_enrolled: false,
                valid_id: "seamanBook",
                fields: Object.fromEntries(COMPARED.map((field) => [field, "MATCHES"])),
            },
        })
        setUp({application, evaluation: decided(2)})
        expect(summary()["ID type"]).toBe("Seafarer's Book")
        expect(
            screen.getByText("The current simulation gives the same decision as the recorded one.")
        ).toBeInTheDocument()
    })

    it("explains first that an unreadable decision is simulated with the sample", () => {
        setUp({application: carol({inputs: {voter_found: "yes"}})})
        expect(heading(3, "Simulating the enrollment of Carol Sample")).toBeInTheDocument()
        const notice = screen.getByText(
            "The recorded decision of this enrollment can't be read, so the simulation uses the sample enrollment."
        )
        const section = heading(4, "Sample enrollment — not their recorded details")
        expect(
            notice.compareDocumentPosition(section) & Node.DOCUMENT_POSITION_FOLLOWING
        ).toBeTruthy()
        expect(screen.queryByText(/^Rule 2 of version 1/)).toBeNull()
        expect(screen.queryByText(/recorded one/)).toBeNull()
        fireEvent.click(screen.getByRole("button", {name: "Try other details"}))
        expect(screen.getByRole("button", {name: "Fill in the sample"})).toBeInTheDocument()
    })

    it("says that other details than an unreadable decision's started from the sample", () => {
        setUp({
            application: carol({inputs: {voter_found: "yes"}}),
            applied: {...sampleEnrollment(), identity: EIdentityMethod.MANUAL_ENTRY},
        })
        expect(heading(4, "Other details · in the simulation")).toBeInTheDocument()
        expect(screen.getByText("Started from the sample.")).toBeInTheDocument()
        expect(screen.queryByText("The enrollment keeps its recorded details.")).toBeNull()
    })

    it("names an enrollment without a name", () => {
        setUp({application: carol({}, "")})
        expect(heading(3, "Simulating an enrollment")).toBeInTheDocument()
        expect(screen.getByRole("button", {name: "Back to the enrollment"})).toBeInTheDocument()
    })

    it("adds the status an officer set since and an approval that several voters fit", () => {
        setUp({
            application: {
                ...carol({accepted_candidates: 2, invariant: "MANUAL_ENTRY_NOT_ACCEPTED"}),
                status: ACCEPTED,
            },
        })
        const line = screen.getByText(/^Rule 2 of version 1/)
        expect(line.parentElement).toHaveTextContent(
            "Several registry voters fit. Its status is now Approved."
        )
    })
})

describe("the simulation", () => {
    it.each([
        [{state: EExampleResult.PENDING}, "Working out which rule decides the simulation…"],
        [{state: EExampleResult.ERROR}, "The simulation could not be run."],
    ])("selects no rule while %j", (evaluation, text) => {
        setUp({evaluation})
        expect(screen.getByRole("status", {name: "Current simulation"})).toHaveTextContent(text)
        expect(screen.queryByRole("button", {name: /^Go to/})).toBeNull()
    })

    it("lists why the rules can't be tried", () => {
        setUp({
            evaluation: {
                state: EExampleResult.INVALID,
                result: {
                    rule: null,
                    decision: null,
                    reason: null,
                    invariant: null,
                    errors: [
                        {code: "NO_COMPARED_FIELDS", rule: null, field: null},
                        {code: "ACCEPTS_MANUAL_ENTRY", rule: 2, field: null},
                    ],
                },
            },
        })
        expect(screen.getByRole("status", {name: "Current simulation"})).toHaveTextContent(
            "Fix these rules to run the simulation:" +
                "Choose at least one detail to compare with the registry." +
                "Rule 2: Enrollments whose identity was typed by hand can't be approved automatically."
        )
    })

    it("names the deciding rule and goes to it", () => {
        const props = setUp({evaluation: decided(3, ACCEPTED)})
        expect(screen.getByRole("status", {name: "Current simulation"})).toHaveTextContent(
            "In the simulation, rule 3 decides."
        )
        fireEvent.click(screen.getByRole("button", {name: "Go to rule 3"}))
        expect(props.onGoToRule).toHaveBeenCalledWith(3)
    })

    it("names the last rule when no rule fits", () => {
        const props = setUp({evaluation: decided(null, PENDING)})
        expect(screen.getByRole("status", {name: "Current simulation"})).toHaveTextContent(
            "In the simulation, no rule fits: the last rule decides."
        )
        fireEvent.click(screen.getByRole("button", {name: "Go to the last rule"}))
        expect(props.onGoToRule).toHaveBeenCalledWith(null)
    })

    it("doesn't say a rule is highlighted while none is", () => {
        setUp({evaluation: {state: EExampleResult.PENDING}})
        fireEvent.click(screen.getByRole("button", {name: "Edit example"}))
        expect(screen.queryByText(/highlighted rule still decides/)).toBeNull()
    })
})
