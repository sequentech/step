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
 * The load-bearing case is the last one. `startScreen.*` lives in *voting-portal*'s
 * catalogue and not in `ui-essentials`, so a layout that translated for itself would
 * draw raw keys everywhere else it was used. That is not hypothetical: the wizard's
 * preview shipped raw `selectElection.*` keys for exactly this reason. The wording is
 * a prop, and the test that matters is that nothing here invents it.
 */

import {ThemeProvider} from "@mui/material/styles"
import {render as mount, screen} from "@testing-library/react"
import React from "react"

import theme from "../services/theme"
import {StartLayout, START_WORDING_EN} from "./StartLayout"

const render = (ui: React.ReactElement) =>
    mount(<ThemeProvider theme={theme}>{ui}</ThemeProvider>)

describe("the start screen's arrangement", () => {
    it("draws the title, the instructions and three steps", () => {
        render(<StartLayout title="Claustro 2026" />)

        expect(screen.getByText("Claustro 2026")).toBeInTheDocument()
        expect(screen.getByText("Instructions")).toBeInTheDocument()
        for (const step of START_WORDING_EN.steps) {
            expect(screen.getByText(step.title)).toBeInTheDocument()
        }
    })

    it("leaves the description out when there is none", () => {
        const {container} = render(<StartLayout title="Claustro 2026" />)
        // The election's own description is optional, and an empty paragraph in its
        // place is a gap somebody reads as a rendering fault.
        expect(container.textContent).not.toContain("undefined")

        render(
            <StartLayout title="Otra" description={<span>What this is about</span>} />
        )
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
        expect(
            screen.getByRole("button", {name: "Start Voting"})
        ).toBeInTheDocument()
    })

    it("says only what it is handed, so it cannot draw a raw key", () => {
        render(
            <StartLayout
                title="Claustro 2026"
                wording={{
                    instructionsTitle: "Cómo votar",
                    instructionsDescription: "Siga estos pasos:",
                    steps: [{title: "1. Elija", description: "Marque su opción."}],
                }}
            />
        )

        expect(screen.getByText("Cómo votar")).toBeInTheDocument()
        expect(screen.getByText("1. Elija")).toBeInTheDocument()
        // The English default is nowhere on the page, and neither is a key: a layout
        // that fell back per-string would mix the two, and one that translated for
        // itself would print `startScreen.instructionsTitle`.
        expect(screen.queryByText("Instructions")).not.toBeInTheDocument()
        expect(document.body.textContent).not.toContain("startScreen.")
    })
})
