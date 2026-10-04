// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {ThemeProvider} from "@mui/material/styles"
import {act, render, screen, within} from "@testing-library/react"
import {theme} from "@sequentech/ui-essentials"
import useMediaQuery from "@mui/material/useMediaQuery"
import {I18nextProvider} from "react-i18next"

import {aCandidate, aContest, i18n} from "../../testing/ballotHarness"
import {PRESIDENT, SLATES, TRUSTEES} from "../../testing/slateFixtures"
import {resolveSlates} from "../../services/Slates"
import {ISlateChooserProps, SlateChooser} from "./SlateChooser"

jest.mock("@mui/material/useMediaQuery")
const mockedUseMediaQuery = jest.mocked(useMediaQuery)

const RESOLVED = resolveSlates(SLATES, [PRESIDENT, TRUSTEES])

const mount = (props: Partial<ISlateChooserProps> = {}) =>
    render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <SlateChooser slates={RESOLVED} defaultLanguage="en" {...props} />
            </ThemeProvider>
        </I18nextProvider>
    )

const cardOf = (name: string): HTMLElement => {
    const card = screen.getByRole("heading", {level: 3, name}).closest("li")
    if (!card) {
        throw new Error(`no card for ${name}`)
    }
    return card
}

const officesOf = (name: string): Array<string | null> =>
    within(cardOf(name))
        .getAllByRole("heading", {level: 4})
        .map((heading) => heading.textContent)

const officeOf = (name: string, contestId: string): HTMLElement => {
    const office = cardOf(name).querySelector<HTMLElement>(`[data-contest-id="${contestId}"]`)
    if (!office) {
        throw new Error(`no ${contestId} in ${name}`)
    }
    return office
}

beforeEach(() => mockedUseMediaQuery.mockReturnValue(false))

afterEach(async () => {
    await act(async () => {
        await i18n.changeLanguage("en")
    })
})

describe("the slates of a ballot", () => {
    it("are introduced by a heading", () => {
        mount()

        expect(screen.getByRole("region", {name: "Slates"})).toBeInTheDocument()
    })

    it("show their names in the configured order", () => {
        mount()

        const names = screen
            .getAllByRole("heading", {level: 3})
            .map((heading) => heading.textContent)
        expect(names).toEqual(["Forward Together", "Members First", "Independent Voices"])
    })

    it("list their candidates under each office", () => {
        mount()

        const forward = within(cardOf("Forward Together"))
        expect(
            within(forward.getByRole("list", {name: "Forward Together candidates for President"}))
                .getAllByRole("listitem")
                .map((item) => item.textContent)
        ).toEqual(["Jordan Ellis"])
        expect(
            within(forward.getByRole("list", {name: "Forward Together candidates for Trustees"}))
                .getAllByRole("listitem")
                .map((item) => item.textContent)
        ).toEqual(["Rowan Scott", "Charlie Kim"])
    })

    it("render nothing when the ballot has no slate", () => {
        const {container} = mount({slates: {...RESOLVED, slates: [], contests: []}})

        expect(container).toBeEmptyDOMElement()
    })
})

describe("the offices of the slates on desktop", () => {
    it("are the same in every card, in ballot order", () => {
        mount()

        expect(officesOf("Forward Together")).toEqual(["President", "Trustees"])
        expect(officesOf("Independent Voices")).toEqual(["President", "Trustees"])
    })

    it("say when a slate has no candidate for one of them", () => {
        mount()

        const president = officeOf("Independent Voices", "president")
        expect(president).toHaveClass("slate-contest-empty")
        expect(president).toHaveTextContent("No candidate")
        expect(within(president).queryByRole("list")).toBeNull()
        expect(within(cardOf("Forward Together")).queryByText("No candidate")).toBeNull()
    })

    it("are on the same row in every card", () => {
        mount()

        const row = (name: string, contestId: string) => officeOf(name, contestId).style.gridRow
        expect(row("Forward Together", "president")).not.toBe("")
        expect(row("Forward Together", "president")).toBe(row("Independent Voices", "president"))
        expect(row("Forward Together", "trustees")).toBe(row("Independent Voices", "trustees"))
        expect(row("Forward Together", "president")).not.toBe(row("Forward Together", "trustees"))
    })

    it("leave out a contest no slate has candidates for", () => {
        const secretary = aContest({
            id: "secretary",
            name: "Secretary",
            candidates: [aCandidate("i-secretary", "Quinn Parker", {contest_id: "secretary"})],
        })
        mount({slates: resolveSlates(SLATES, [PRESIDENT, secretary, TRUSTEES])})

        expect(officesOf("Independent Voices")).toEqual(["President", "Trustees"])
    })
})

describe("the offices of the slates on a phone", () => {
    beforeEach(() => mockedUseMediaQuery.mockReturnValue(true))

    it("leave out an office a slate has no candidate for", () => {
        mount()

        expect(officesOf("Forward Together")).toEqual(["President", "Trustees"])
        expect(officesOf("Independent Voices")).toEqual(["Trustees"])
        expect(screen.queryByText("No candidate")).toBeNull()
    })

    it("are not placed on shared rows", () => {
        mount()

        expect(officeOf("Forward Together", "trustees").style.gridRow).toBe("")
    })
})

describe("the names of the slates", () => {
    it("are named in the voter's language, else in the default language", async () => {
        mount()

        await act(async () => {
            await i18n.changeLanguage("es")
        })

        expect(screen.getByRole("heading", {level: 3, name: "Adelante Juntos"})).toBeInTheDocument()
        expect(screen.getByRole("heading", {level: 3, name: "Members First"})).toBeInTheDocument()
    })
})

describe("what other features add to a slate", () => {
    it("is placed in the card of each slate", () => {
        mount({
            renderCoverage: (slate) => `coverage of ${slate.id}`,
            renderSummary: (slate) => `summary of ${slate.id}`,
            renderActions: (slate) => <button>{`choose ${slate.id}`}</button>,
        })

        const voices = within(cardOf("Independent Voices"))
        expect(voices.getByText("coverage of voices")).toBeInTheDocument()
        expect(voices.getByText("summary of voices")).toBeInTheDocument()
        expect(voices.getByRole("button", {name: "choose voices"})).toBeInTheDocument()
    })

    it("can replace the row of a member and wrap the lists", () => {
        mount({
            renderMember: (slate, slateContest, candidate) => (
                <li>{`${candidate.name} (${slateContest.contest.id})`}</li>
            ),
            renderMemberLists: (slate, lists) => (
                <details open data-testid={`lists-${slate.id}`}>
                    {lists}
                </details>
            ),
        })

        const lists = within(screen.getByTestId("lists-voices"))
        expect(lists.getByRole("listitem")).toHaveTextContent("Harper Lane (trustees)")
    })
})
