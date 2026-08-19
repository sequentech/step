// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The row of buttons under the ballot.
 *
 * Lifted out of the portal's `VotingScreen`, where it was `ActionButtons`. The Election
 * Architect's Ballot Preview had invented its own row — one button, labelled "Back",
 * with no chevron and no Clear beside it — which is the sort of difference that only
 * shows up when somebody compares the preview to the real thing.
 *
 * Two of these cases pin things that look like mistakes and are not: there are two
 * Clear buttons, one per breakpoint, and Back is a frame around a button rather than a
 * button, because the portal navigates with a router link and a preview has no router.
 */

import {screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import React from "react"

import {BallotActions} from "./BallotActions"
import {catalogue, inAHost, OTHER_WORDS, PORTAL_WORDS} from "./testCatalogue"

const render = inAHost
const WORDS = PORTAL_WORDS.votingScreen

describe("the buttons under a ballot", () => {
    it("says what the catalogue says, on the portal's own keys", () => {
        // `votingScreen.backButton`, `.clearButton`, `.reviewButton` — the paths clients
        // override, translated by this component rather than copied into it. The words
        // asserted are the test catalogue's, so this can only pass through i18n.
        render(<BallotActions />)

        expect(screen.getByText(WORDS.backButton)).toBeInTheDocument()
        expect(screen.getByText(WORDS.reviewButton)).toBeInTheDocument()
        expect(screen.getAllByText(WORDS.clearButton)).toHaveLength(2)
    })

    it("follows the catalogue into another language", () => {
        // The defect `EA-F2-053` was reported for. This file used to hold an English
        // copy of these three and the wizard's preview read the copy — so a Spanish
        // preview of a Spanish election said "Clear choices".
        render(<BallotActions />, catalogue(OTHER_WORDS, "es"))

        expect(screen.getByText("Atrás")).toBeInTheDocument()
        expect(screen.getByText("Siguiente")).toBeInTheDocument()
        expect(screen.getAllByText("Borrar")).toHaveLength(2)
        expect(screen.queryByText("Back")).toBeNull()
    })

    it("invents no English of its own", () => {
        // A host with no catalogue draws the keys. That is the honest failure — it says
        // *supply the catalogue* — where a hard-coded English word would quietly show
        // the platform's wording to a client who had translated it.
        render(<BallotActions />, catalogue({}, "en"))

        expect(screen.getByText("votingScreen.backButton")).toBeInTheDocument()
        expect(screen.queryByText("Back")).toBeNull()
        expect(screen.queryByText("Clear choices")).toBeNull()
    })


    it("draws Clear twice, one per breakpoint", () => {
        // Not a duplicate to tidy: on a phone Clear is full-width above the row, and
        // on a wide screen it sits between Back and Next. Both are in the tree and CSS
        // picks.
        render(<BallotActions />)

        expect(screen.getAllByText(WORDS.clearButton)).toHaveLength(2)
    })

    it("puts a chevron on each of Back and Next", () => {
        const {container} = render(<BallotActions />)

        // `Icon` renders an `svg`; there is one at each end of the row.
        expect(container.querySelectorAll("svg").length).toBeGreaterThanOrEqual(2)
    })

    it("navigates Back through whatever the host gave it", () => {
        render(<BallotActions backComponent="a" backTo="/somewhere" />)

        // `Box` forwards `component`, so the frame is the host's element. The portal
        // passes its router's `Link`; this test passes an anchor, which is the same
        // shape without pulling a router into a component library's tests.
        expect(document.querySelector("a")).not.toBeNull()
    })

    it("frames Back in a plain element when there is nowhere to navigate", () => {
        render(<BallotActions />)

        expect(document.querySelector("a")).toBeNull()
        expect(screen.getByText(WORDS.backButton)).toBeInTheDocument()
    })

    it("does not put a stray `to` attribute on that plain element", () => {
        // `to` means something to a router link and nothing to a `div`; React would
        // warn about it, and a warning in a voter's console is a bug report waiting.
        render(<BallotActions backTo="/ignored" />)

        expect(document.querySelector("[to]")).toBeNull()
    })

    it("reports Back, Clear and Next to the host", async () => {
        const back = jest.fn()
        const clear = jest.fn()
        const next = jest.fn()
        render(<BallotActions onBack={back} onClear={clear} onNext={next} />)

        await userEvent.click(screen.getByText(WORDS.backButton))
        await userEvent.click(screen.getAllByText(WORDS.clearButton)[0])
        await userEvent.click(screen.getByText(WORDS.reviewButton))

        expect(back).toHaveBeenCalled()
        expect(clear).toHaveBeenCalled()
        expect(next).toHaveBeenCalled()
    })

    it("clears from either of the two Clear buttons", async () => {
        const clear = jest.fn()
        render(<BallotActions onClear={clear} />)

        for (const button of screen.getAllByText(WORDS.clearButton)) {
            await userEvent.click(button)
        }

        expect(clear).toHaveBeenCalledTimes(2)
    })

    it("refuses Next while a contest is over-voted, and nothing else", () => {
        render(<BallotActions disableNext />)

        expect(screen.getByText(WORDS.reviewButton).closest("button")).toBeDisabled()
        expect(screen.getByText(WORDS.backButton).closest("button")).not.toBeDisabled()
        expect(screen.getAllByText(WORDS.clearButton)[0].closest("button")).not.toBeDisabled()
    })

    it("draws every button dead in a preview", () => {
        // The preview shows the row a voter will meet; there is no ballot to clear and
        // nowhere to go next, so a live-looking control would be a lie.
        render(<BallotActions inert />)

        for (const button of document.querySelectorAll("button")) {
            expect(button).toBeDisabled()
        }
    })
})
