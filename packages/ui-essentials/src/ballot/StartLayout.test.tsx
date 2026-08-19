// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What the start screen puts on the page, and where its words come from.
 *
 * `StartLayout` was lifted out of the voting portal's `StartScreen` so the Election
 * Architect's preview can render the screen a voter actually meets rather than a
 * drawing of it — the same reason `ReviewLayout` and `ConfirmationLayout` were lifted.
 * The arrangement is now a contract between two applications instead of the inside of
 * one route, so these tests pin it.
 *
 * **Where the words come from, since this got it wrong once.** `startScreen.*` lives in
 * `voting-portal/src/translations/<lng>.ts`, and this layout translates those paths — it
 * does not take the wording as a prop and it holds no English of its own. It did hold
 * eight strings for a while, as `START_WORDING_EN`, on the theory that a host without
 * that catalogue would draw raw keys. Both hosts have it (the wizard vendors the
 * portal's), and the copy meant a Spanish preview showed English. `EA-F2-053`.
 *
 * So the tests supply a catalogue, as `Candidate.test.tsx` does, and its words are
 * deliberately not the shipped English: an assertion on "Instructions" would pass
 * against a component that hard-coded it.
 */

import {screen} from "@testing-library/react"
import React from "react"

import {StartLayout} from "./StartLayout"
import {catalogue, inAHost, OTHER_WORDS, PORTAL_WORDS} from "./testCatalogue"

const render = inAHost
const WORDS = PORTAL_WORDS.startScreen

describe("the start screen's arrangement", () => {
    it("draws the title, the instructions and three steps", () => {
        render(<StartLayout title="Claustro 2026" />)

        expect(screen.getByText("Claustro 2026")).toBeInTheDocument()
        expect(screen.getByText(WORDS.instructionsTitle)).toBeInTheDocument()
        expect(screen.getByText(WORDS.instructionsDescription)).toBeInTheDocument()
        for (const step of [WORDS.step1Title, WORDS.step2Title, WORDS.step3Title]) {
            expect(screen.getByText(step)).toBeInTheDocument()
        }
    })

    it("leaves the description out when there is none", () => {
        const {container} = render(<StartLayout title="Claustro 2026" />)
        // The election's own description is optional, and an empty paragraph in its
        // place is a gap somebody reads as a rendering fault.
        expect(container.textContent).not.toContain("undefined")

        render(<StartLayout title="Otra" description={<span>What this is about</span>} />)
        expect(screen.getByText("What this is about")).toBeInTheDocument()
    })

    it("frames whatever the caller puts above and below it", () => {
        // The stepper and the action row belong to the caller: they navigate and they
        // act, and a layout that acts is a layout that needs a store.
        render(
            <StartLayout
                title="Claustro 2026"
                above={<div data-testid="the-stepper" />}
                below={<button type="button">Start Voting</button>}
            />
        )

        expect(screen.getByTestId("the-stepper")).toBeInTheDocument()
        expect(screen.getByRole("button", {name: "Start Voting"})).toBeInTheDocument()
    })

    it("follows the catalogue into another language", () => {
        // The defect this file's own doc describes: with the English copy in place, a
        // Spanish election previewed in Spanish showed English instructions.
        render(<StartLayout title="Claustro 2026" />, catalogue(OTHER_WORDS, "es"))

        expect(screen.getByText("Cómo funciona")).toBeInTheDocument()
        expect(screen.getByText("Uno")).toBeInTheDocument()
        expect(screen.queryByText(WORDS.instructionsTitle)).toBeNull()
    })

    it("invents no English of its own", () => {
        // With no catalogue it draws the keys, which says *supply the catalogue*. A
        // hard-coded English word here would instead show the platform's wording to a
        // client who had translated it, which is what happened.
        render(<StartLayout title="Claustro 2026" />, catalogue({}, "en"))

        expect(screen.getByText("startScreen.instructionsTitle")).toBeInTheDocument()
        expect(screen.queryByText("Instructions")).toBeNull()
        expect(document.body.textContent).not.toContain("Please follow these steps")
    })

    it("frames the breadcrumb 48px under the header, as its sibling layouts do", () => {
        // The portal's route used to do this framing itself, through `above`, which
        // left the one measurement written down in every caller — and the wizard's
        // preview wrote it down as 16px, outside the screen.
        render(<StartLayout title="Claustro 2026" steps={<div data-testid="crumbs" />} />)

        expect(screen.getByTestId("crumbs").parentElement).toHaveStyle({marginTop: "48px"})
    })

    it("still takes anything else above the title, unframed", () => {
        render(<StartLayout title="Claustro 2026" above={<div data-testid="something-else" />} />)

        expect(screen.getByTestId("something-else")).toBeInTheDocument()
    })

    it("puts the breadcrumb before whatever else is above the title", () => {
        render(
            <StartLayout
                title="Claustro 2026"
                steps={<div data-testid="crumbs" />}
                above={<div data-testid="something-else" />}
            />
        )

        expect(
            screen
                .getByTestId("crumbs")
                .compareDocumentPosition(screen.getByTestId("something-else"))
        ).toBe(Node.DOCUMENT_POSITION_FOLLOWING)
    })
})
