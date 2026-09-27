// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import i18next, {type i18n as I18n} from "i18next"
import {I18nextProvider} from "react-i18next"

import english from "../../../../ui-core/src/translations/en"
import spanish from "../../../../ui-core/src/translations/es"
import {ProblemList} from "./ProblemList"
import {problemSentence} from "./sentence"
import {readProblems, type Problem, type ProblemReport} from "./types"

const problem = (overrides: Partial<Problem> = {}): Problem => ({
    severity: "error",
    code: "invalid_value",
    path: "contests[0].max_votes",
    message: "the fallback sentence",
    ...overrides,
})

const report = (...problems: Problem[]): ProblemReport => ({problems})

/** An i18next with the real catalogue, the way a portal's `initializeLanguages` loads it. */
const withCatalogue = async (lng: "en" | "es"): Promise<I18n> => {
    const instance = i18next.createInstance()
    await instance.init({
        lng,
        fallbackLng: "en",
        resources: {
            en: {translation: english.translations},
            es: {translation: spanish.translations},
        },
        interpolation: {escapeValue: false},
    })
    return instance
}

/** No catalogue at all: every sentence comes from a `defaultValue`. */
const bare = i18next.createInstance()
void bare.init({lng: "en", resources: {}, interpolation: {escapeValue: false}})

const show = (element: React.ReactElement, i18n: I18n = bare) =>
    render(<I18nextProvider i18n={i18n}>{element}</I18nextProvider>)

describe("ProblemList", () => {
    it("says so when there is nothing wrong, rather than showing nothing", () => {
        // An empty panel reads as "it did not run".
        show(<ProblemList report={report()} />)
        expect(screen.getByText(/would import/i)).toBeInTheDocument()
    })

    it("uses the caller's words for an empty report when it has some", () => {
        show(<ProblemList report={report()} emptyMessage="All clear" />)
        expect(screen.getByText("All clear")).toBeInTheDocument()
    })

    it("separates what stops an import from what merely looks wrong", () => {
        show(
            <ProblemList
                report={report(problem(), problem({severity: "warning", code: "permission_label"}))}
            />
        )
        expect(screen.getByTestId("problem-group-error")).toBeInTheDocument()
        expect(screen.getByTestId("problem-group-warning")).toBeInTheDocument()
        expect(screen.getAllByTestId("problem")).toHaveLength(2)
    })

    it("keeps the path where an engineer can read it, without printing it", () => {
        const goTo = jest.fn()
        show(
            <ProblemList report={report(problem({path: "sheet 'Contests' row 4"}))} onGoTo={goTo} />
        )
        expect(screen.queryByText("sheet 'Contests' row 4")).not.toBeInTheDocument()
        const link = screen.getByTestId("goto-sheet 'Contests' row 4")
        expect(link).toHaveAttribute("title", "sheet 'Contests' row 4")
        fireEvent.click(link)
        expect(goTo).toHaveBeenCalledWith("sheet 'Contests' row 4")
    })

    it("offers no link when the host has nowhere to go", () => {
        show(<ProblemList report={report(problem())} />)
        expect(screen.queryByRole("button")).not.toBeInTheDocument()
    })

    it("shows the external_id rather than only a regenerated uuid", () => {
        show(<ProblemList report={report(problem({external_id: "president"}))} />)
        expect(screen.getByText("president")).toBeInTheDocument()
    })

    it("does not show the code, which is a category and not a sentence", () => {
        show(<ProblemList report={report(problem({code: "area_cycle"}))} />)
        expect(screen.queryByText("area_cycle")).not.toBeInTheDocument()
    })

    it("counts in the heading, in the singular for one", () => {
        const {unmount} = show(<ProblemList report={report(problem(), problem(), problem())} />)
        expect(screen.getByText(/3 errors/i)).toBeInTheDocument()
        unmount()
        show(<ProblemList report={report(problem())} />)
        expect(screen.getByText(/1 error/i)).toBeInTheDocument()
        expect(screen.queryByText(/1 errors/i)).not.toBeInTheDocument()
    })

    it("shows warnings as blocking under strict, because they are", () => {
        const warnings = report(problem({severity: "warning"}))
        const {rerender} = show(<ProblemList report={warnings} />)
        expect(screen.getByTestId("problem-group-warning")).toBeInTheDocument()

        rerender(
            <I18nextProvider i18n={bare}>
                <ProblemList report={warnings} strict />
            </I18nextProvider>
        )
        expect(screen.getByTestId("problem-group-error")).toBeInTheDocument()
        expect(screen.getByText(/strict mode/i)).toBeInTheDocument()
    })

    it("renders two problems that share a code and a path", () => {
        show(<ProblemList report={report(problem(), problem())} />)
        expect(screen.getAllByTestId("problem")).toHaveLength(2)
    })
})

describe("what a problem says", () => {
    it("shows the core's own English when the problem has no name", () => {
        show(<ProblemList report={report(problem({message: "no election has id 'x'"}))} />)
        expect(screen.getByText("no election has id 'x'")).toBeInTheDocument()
    })

    it("shows the core's English when the name has no translation either", async () => {
        // `defaultValue` rather than a branch, so a key nobody has written yet can
        // never render blank or render the key itself back at somebody.
        show(
            <ProblemList
                report={report(problem({id: "nothing.translated-this", message: "the fallback"}))}
            />,
            await withCatalogue("es")
        )
        expect(screen.getByText("the fallback")).toBeInTheDocument()
    })

    it("puts the specifics the core sent into the translated sentence", async () => {
        // With the real catalogue, so this fails if the key is wrong, if the
        // placeholder is spelled differently from the detail the core sends, or if
        // `details` never reaches `t`.
        show(
            <ProblemList
                report={report(
                    problem({
                        id: "trustee.email-malformed",
                        message: "'not an address' is not an email address",
                        details: {email: "not an address"},
                    })
                )}
            />,
            await withCatalogue("es")
        )
        expect(screen.getByTestId("problem")).toHaveTextContent(
            "Correo no válido — «not an address» no lo parece."
        )
    })

    it("says what an import refused, in the administrator's language", async () => {
        show(
            <ProblemList
                report={report(
                    problem({
                        id: "voters.vote-weight-out-of-range",
                        code: "invalid_value",
                        path: "row 4 column 'vote-weight'",
                        message: "the vote weight 0 on row 4 must be between 1 and 100",
                        details: {row: "4", value: "0", min: "1", max: "100"},
                    })
                )}
            />,
            await withCatalogue("es")
        )
        expect(screen.getByText(/1 error/)).toBeInTheDocument()
        expect(screen.getByTestId("problem")).toHaveTextContent(
            spanish.translations.problems.messages.voters["vote-weight-out-of-range"].text
                .replace("{{value}}", "0")
                .replace("{{row}}", "4")
                .replace("{{min}}", "1")
                .replace("{{max}}", "100")
        )
    })

    it("survives a problem whose details the core left out entirely", async () => {
        // `details` is `skip_serializing_if` in Rust: absent, not `{}`.
        show(
            <ProblemList
                report={report(problem({id: "contest.no-candidates", message: "no candidates"}))}
            />,
            await withCatalogue("en")
        )
        expect(screen.getByTestId("problem")).toBeInTheDocument()
    })
})

describe("problemSentence", () => {
    it("links the lead and leaves the rest as text", async () => {
        const i18n = await withCatalogue("en")
        const sentence = problemSentence(
            (key, options) => i18n.t(key, options),
            problem({id: "file.cannot-decrypt"})
        )
        expect(sentence.lead).toBe(
            english.translations.problems.messages.file["cannot-decrypt"].lead
        )
        expect(sentence.lead + sentence.rest).toBe(sentence.text)
    })

    it("interpolates the core's message where a sentence quotes it", () => {
        const sentence = problemSentence(
            (_, options) => `Unreadable — ${options.message as string}`,
            problem({id: "any.thing", message: "bad row"})
        )
        expect(sentence.text).toBe("Unreadable — bad row")
    })

    it("makes the whole sentence the lead when a translation's lead is not a prefix", () => {
        const sentence = problemSentence(
            (key) => (key.endsWith(".lead") ? "Something else" : "The sentence — in full"),
            problem({id: "any.thing"})
        )
        expect(sentence).toEqual({
            text: "The sentence — in full",
            lead: "The sentence — in full",
            rest: "",
        })
    })
})

describe("readProblems", () => {
    it("reads the problems a server sent", () => {
        expect(readProblems([problem()])).toEqual([problem()])
    })

    it("is nothing for nothing, so a caller falls back to its plain error", () => {
        expect(readProblems(undefined)).toBeUndefined()
        expect(readProblems(null)).toBeUndefined()
        expect(readProblems("oops")).toBeUndefined()
        expect(readProblems([])).toBeUndefined()
    })

    it("drops what is not a problem rather than rendering undefined", () => {
        expect(readProblems([{severity: "fatal"}, 3, null, problem()])).toEqual([problem()])
    })
})
