// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {ThemeProvider} from "@mui/material/styles"
import {act, render, screen, within} from "@testing-library/react"
import {EMobileCandidateLists, ISlatesConfig} from "@sequentech/ui-core"
import {theme} from "@sequentech/ui-essentials"
import {I18nextProvider} from "react-i18next"

import {i18n} from "../../testing/ballotHarness"
import {aSlateBallotStyle, SLATES} from "../../testing/slateFixtures"
import {IBallotSlates, resolveBallotStyleSlates} from "../../services/Slates"
import {SlateChooser} from "./SlateChooser"
import catalanTranslation from "../../translations/cat"
import englishTranslation from "../../translations/en"
import spanishTranslation from "../../translations/es"
import basqueTranslation from "../../translations/eu"
import frenchTranslation from "../../translations/fr"
import galicianTranslation from "../../translations/gl"
import dutchTranslation from "../../translations/nl"
import tagalogTranslation from "../../translations/tl"

const TRANSLATIONS = {
    cat: catalanTranslation,
    es: spanishTranslation,
    eu: basqueTranslation,
    fr: frenchTranslation,
    gl: galicianTranslation,
    nl: dutchTranslation,
    tl: tagalogTranslation,
}

const FULL_AND_PARTIAL: ISlatesConfig = {
    version: 1,
    mobile_candidate_lists: EMobileCandidateLists.COLLAPSED,
    slates: [
        {
            id: "forward",
            name: {en: "Forward Together"},
            members: {president: ["f-president"], trustees: ["f-t1", "f-t2", "i-t1"]},
        },
        ...SLATES.slates.slice(1),
    ],
}

const resolve = (config: ISlatesConfig): IBallotSlates => {
    const slates = resolveBallotStyleSlates(aSlateBallotStyle(JSON.stringify(config)).ballot_eml)
    if (!slates) {
        throw new Error("the ballot has no slates")
    }
    return slates
}

const mount = (config: ISlatesConfig = FULL_AND_PARTIAL) =>
    render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <SlateChooser slates={resolve(config)} defaultLanguage="en" />
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

const coverageOf = (name: string): string =>
    cardOf(name).querySelector(".slate-coverage")?.textContent ?? ""

const officeOf = (name: string, contestId: string): HTMLElement => {
    const office = cardOf(name).querySelector<HTMLElement>(`[data-contest-id="${contestId}"]`)
    if (!office) {
        throw new Error(`no ${contestId} in ${name}`)
    }
    return office
}

afterEach(async () => {
    await act(async () => {
        await i18n.changeLanguage("en")
    })
})

describe("what a slate covers", () => {
    it("is a full slate when it has a candidate for every seat", () => {
        mount()

        expect(coverageOf("Forward Together")).toBe("Full slate · 4 candidates · 2 offices")
    })

    it("is partial when a seat is left without a candidate", () => {
        mount()

        expect(coverageOf("Members First")).toBe("Partial slate · 2 candidates · 2 offices")
    })

    it("names the only office of a slate that leaves the others", () => {
        mount()

        expect(coverageOf("Independent Voices")).toBe("Trustees only · 1 candidate · 1 office")
    })
})

describe("the wording of what a slate covers", () => {
    it.each(Object.entries(TRANSLATIONS))("is complete in %s", (language, bundle) => {
        const {slates} = bundle.translations

        expect(Object.keys(slates.coverage).sort()).toEqual(
            Object.keys(englishTranslation.translations.slates.coverage).sort()
        )
        expect(slates.coverage.singleContest).toContain("{{contest}}")
        expect(slates.noCandidate).not.toBe("")
    })
})

describe("an office a slate has no candidate for", () => {
    it("is kept in its place, without a candidate", () => {
        mount()

        const voices = within(cardOf("Independent Voices"))
        expect(voices.getAllByRole("heading", {level: 4}).map((h) => h.textContent)).toEqual([
            "President",
            "Trustees",
        ])
        const president = officeOf("Independent Voices", "president")
        expect(president).toHaveTextContent("No candidate")
        expect(within(president).queryByRole("list")).not.toBeInTheDocument()
        expect(within(president).queryByRole("listitem")).not.toBeInTheDocument()
    })

    it("is marked so that a phone leaves it out", () => {
        mount()

        expect(officeOf("Independent Voices", "president")).toHaveClass("slate-contest-uncovered")
        expect(officeOf("Independent Voices", "trustees")).not.toHaveClass(
            "slate-contest-uncovered"
        )
        expect(officeOf("Forward Together", "president")).not.toHaveClass("slate-contest-uncovered")
    })

    it("does not appear among the contests the slate would select", () => {
        const voices = resolve(FULL_AND_PARTIAL).slates[2]

        expect(voices.contests.map(({contest}) => contest.id)).toEqual(["trustees"])
        expect(voices.coverage?.uncovered_contest_ids).toEqual(["president"])
    })
})

describe("a slate with no candidate the voter can choose", () => {
    it("is not offered", () => {
        const config: ISlatesConfig = {
            ...FULL_AND_PARTIAL,
            slates: [
                ...FULL_AND_PARTIAL.slates,
                {id: "elsewhere", name: {en: "Elsewhere"}, members: {auditors: ["a-1"]}},
            ],
        }
        mount(config)

        expect(screen.queryByRole("heading", {level: 3, name: "Elsewhere"})).not.toBeInTheDocument()
        expect(screen.getAllByRole("heading", {level: 3})).toHaveLength(3)
    })
})
