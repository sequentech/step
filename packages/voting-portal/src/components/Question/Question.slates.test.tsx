// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {act, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import type {ICandidate} from "@sequentech/ui-core"

import {aCandidate, aContest, i18n, marks, mountContest} from "../../testing/ballotHarness"
import {PRESIDENT, SLATES, TRUSTEES} from "../../testing/slateFixtures"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../../ui-core/src/utils/array"),
    ...jest.requireActual("../../../../ui-core/src/services/categoryService"),
    // These helpers call WASM. The fixtures are plain plurality contests, so no
    // ordering, blank detection or acclamation filtering is needed here.
    sortCandidatesInContest: (candidates: ICandidate[]) => candidates,
    checkIsBlank: () => false,
    isEligibleAcclaimedCandidate: () => true,
}))
jest.mock("../../services/BallotService", () => ({
    provideBallotService: () => ({
        isPreferential: () => false,
        getWriteInAvailableCharacters: () => 240,
    }),
}))

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

describe("a candidate without extra details", () => {
    it("is shown by name and slate only", () => {
        mountContest(TRUSTEES, {slates: SLATES})

        const row = rowOf("Rowan Scott")
        expect(row.querySelector(".candidate-description")).toBeNull()
        expect(row.querySelector(".image-box")).toBeEmptyDOMElement()
        expect(row.querySelector(".candidate-link")).toBeNull()
        expect(row).toHaveTextContent(/^Rowan ScottForward Together$/)
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

describe("clearing one contest on a ballot with slates", () => {
    it("is offered, and only when the contest holds a choice", () => {
        mountContest(TRUSTEES, {slates: SLATES})

        expect(screen.getByRole("button", {name: "Clear Trustees"})).toBeDisabled()
    })

    it("removes the contest's choices and says so", async () => {
        mountContest(TRUSTEES, {slates: SLATES, selection: marks(TRUSTEES, {"f-t1": 0, "i-t1": 0})})

        const clear = screen.getByRole("button", {name: "Clear Trustees"})
        expect(clear).toBeEnabled()
        await userEvent.click(clear)

        expect(
            screen.getByRole("checkbox", {name: "Rowan Scott Forward Together"})
        ).not.toBeChecked()
        expect(screen.getByRole("checkbox", {name: "Blair Lewis Independent"})).not.toBeChecked()
        expect(document.querySelector(".contest-clear-status")).toHaveAttribute("role", "status")
        expect(document.querySelector(".contest-clear-status")).toHaveTextContent(
            "Selections cleared for Trustees."
        )
    })

    it("is not offered on a ballot without slates or in review", () => {
        mountContest(TRUSTEES)
        expect(screen.queryByRole("button", {name: "Clear Trustees"})).toBeNull()
    })
})
