// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {act, screen} from "@testing-library/react"

import {aCandidate, aContest, i18n, marks, mountContest} from "../../testing/ballotHarness"
import {PRESIDENT, SLATES, TRUSTEES} from "../../testing/slateFixtures"

const rowOf = (name: string): HTMLElement => {
    const row = screen.getByText(name).closest("li")
    if (!row) {
        throw new Error(`no row for ${name}`)
    }
    return row
}

const labelOf = (name: string): string | null | undefined =>
    rowOf(name).querySelector(".candidate-subtitle")?.textContent

afterEach(async () => {
    await act(async () => {
        await i18n.changeLanguage("en")
    })
})

describe("the slate of a candidate on the ballot", () => {
    it("is shown next to each slate member", () => {
        mountContest(TRUSTEES, {slates: SLATES})

        expect(labelOf("Rowan Scott")).toBe("Forward Together")
        expect(labelOf("Skyler James")).toBe("Members First")
        expect(labelOf("Harper Lane")).toBe("Independent Voices")
    })

    it("says a candidate in no slate is independent", () => {
        mountContest(TRUSTEES, {slates: SLATES})

        expect(labelOf("Blair Lewis")).toBe("Independent")
    })

    it("is part of the name of the candidate's checkbox", () => {
        mountContest(TRUSTEES, {slates: SLATES})

        expect(
            screen.getByRole("checkbox", {name: "Rowan Scott Forward Together"})
        ).toBeInTheDocument()
    })

    it("follows the voter's language, else the default language", async () => {
        mountContest(TRUSTEES, {slates: SLATES})

        await act(async () => {
            await i18n.changeLanguage("es")
        })

        expect(labelOf("Rowan Scott")).toBe("Adelante Juntos")
        expect(labelOf("Skyler James")).toBe("Members First")
    })

    it("is not shown on write-in, blank or invalid options", () => {
        const contest = aContest({
            ...TRUSTEES,
            candidates: [
                ...TRUSTEES.candidates,
                aCandidate("blank", "Blank vote", {
                    contest_id: "trustees",
                    presentation: {is_explicit_blank: true},
                } as never),
            ],
        })
        mountContest(contest, {slates: SLATES})

        expect(labelOf("Blank vote")).toBeUndefined()
    })

    it("is not shown in a contest no slate has candidates in", () => {
        mountContest(aContest({id: "referendum", name: "Referendum"}), {slates: SLATES})

        expect(document.querySelector(".candidate-subtitle")).toBeNull()
    })

    it("is not shown in an election without slates", () => {
        mountContest(PRESIDENT)

        expect(document.querySelector(".candidate-subtitle")).toBeNull()
    })
})

describe("the slate of a selected candidate in review", () => {
    it("is shown next to each selected candidate", () => {
        mountContest(TRUSTEES, {
            slates: SLATES,
            isReview: true,
            selection: marks(TRUSTEES, {"f-t1": 0, "i-t1": 0}),
        })

        expect(labelOf("Rowan Scott")).toBe("Forward Together")
        expect(labelOf("Blair Lewis")).toBe("Independent")
        expect(screen.queryByText("Harper Lane")).not.toBeInTheDocument()
    })
})
