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

import {ThemeProvider} from "@mui/material/styles"
import {render as mount, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import React from "react"

import {BallotActions, BALLOT_ACTIONS_WORDING_EN} from "./BallotActions"
import {theme} from "../services/theme"

const render = (ui: React.ReactElement) => mount(<ThemeProvider theme={theme}>{ui}</ThemeProvider>)

describe("the buttons under a ballot", () => {
    it("offers Back, Clear and Next, in the portal's words", () => {
        render(<BallotActions />)

        expect(screen.getByText(BALLOT_ACTIONS_WORDING_EN.back)).toBeInTheDocument()
        expect(screen.getByText(BALLOT_ACTIONS_WORDING_EN.next)).toBeInTheDocument()
        // "Clear choices", not "Clear selection": the button's word is `clearButton`,
        // and the help dialog is what says "selection".
        expect(BALLOT_ACTIONS_WORDING_EN.clear).toEqual("Clear choices")
    })

    it("takes translated words when the host has them", () => {
        render(<BallotActions wording={{back: "Atrás", clear: "Borrar", next: "Siguiente"}} />)

        expect(screen.getByText("Atrás")).toBeInTheDocument()
        expect(screen.getByText("Siguiente")).toBeInTheDocument()
        expect(screen.getAllByText("Borrar")).toHaveLength(2)
    })

    it("draws Clear twice, one per breakpoint", () => {
        // Not a duplicate to tidy: on a phone Clear is full-width above the row, and
        // on a wide screen it sits between Back and Next. Both are in the tree and CSS
        // picks.
        render(<BallotActions />)

        expect(screen.getAllByText("Clear choices")).toHaveLength(2)
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
        expect(screen.getByText("Back")).toBeInTheDocument()
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

        await userEvent.click(screen.getByText("Back"))
        await userEvent.click(screen.getAllByText("Clear choices")[0])
        await userEvent.click(screen.getByText("Next"))

        expect(back).toHaveBeenCalled()
        expect(clear).toHaveBeenCalled()
        expect(next).toHaveBeenCalled()
    })

    it("clears from either of the two Clear buttons", async () => {
        const clear = jest.fn()
        render(<BallotActions onClear={clear} />)

        for (const button of screen.getAllByText("Clear choices")) {
            await userEvent.click(button)
        }

        expect(clear).toHaveBeenCalledTimes(2)
    })

    it("refuses Next while a contest is over-voted, and nothing else", () => {
        render(<BallotActions disableNext />)

        expect(screen.getByText("Next").closest("button")).toBeDisabled()
        expect(screen.getByText("Back").closest("button")).not.toBeDisabled()
        expect(screen.getAllByText("Clear choices")[0].closest("button")).not.toBeDisabled()
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
