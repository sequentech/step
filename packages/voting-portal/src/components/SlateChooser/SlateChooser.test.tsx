// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {ThemeProvider} from "@mui/material/styles"
import {act, render, screen, within} from "@testing-library/react"
import {theme} from "@sequentech/ui-essentials"
import {I18nextProvider} from "react-i18next"

import {i18n} from "../../testing/ballotHarness"
import {PRESIDENT, SLATES, TRUSTEES} from "../../testing/slateFixtures"
import {resolveSlates} from "../../services/Slates"
import {ISlateChooserProps, SlateChooser} from "./SlateChooser"

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

    it("do not list an office a slate has no candidate for", () => {
        mount()

        const voices = within(cardOf("Independent Voices"))
        expect(voices.getAllByRole("heading", {level: 4}).map((h) => h.textContent)).toEqual([
            "Trustees",
        ])
    })

    it("are named in the voter's language, else in the default language", async () => {
        mount()

        await act(async () => {
            await i18n.changeLanguage("es")
        })

        expect(screen.getByRole("heading", {level: 3, name: "Adelante Juntos"})).toBeInTheDocument()
        expect(screen.getByRole("heading", {level: 3, name: "Members First"})).toBeInTheDocument()
    })

    it("render nothing when the ballot has no slate", () => {
        const {container} = mount({slates: {...RESOLVED, slates: []}})

        expect(container).toBeEmptyDOMElement()
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
