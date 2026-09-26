// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The ballot screen's arrangement, and the class names on it.
 *
 * Lifted out of the portal's `VotingScreen` for the same reason as
 * `ElectionListLayout`: a client's stylesheet is written against `voting-screen`,
 * `stepper-box`, `title-container`, `selected-election-title` and `description`, and
 * against them nested this way. The Election Architect's preview draws this tree so
 * that CSS lands where it is aimed.
 */

import {ThemeProvider} from "@mui/material/styles"
import {render as mount, screen} from "@testing-library/react"
import React from "react"

import {BallotScreenLayout} from "./BallotScreenLayout"
import {theme} from "../services/theme"

const render = (ui: React.ReactElement) => mount(<ThemeProvider theme={theme}>{ui}</ThemeProvider>)

const asThePortalCallsIt = (props: Partial<React.ComponentProps<typeof BallotScreenLayout>> = {}) =>
    render(
        <BallotScreenLayout
            steps={<div data-testid="the-breadcrumb" />}
            title="Board of Directors 2027"
            description="Choose up to three."
            actions={<div data-testid="the-actions" />}
            {...props}
        >
            <div data-testid="a-contest" />
        </BallotScreenLayout>
    )

describe("the screen where a voter marks a ballot", () => {
    it("shows the election's name, its description and the contests", () => {
        asThePortalCallsIt()

        expect(screen.getByText("Board of Directors 2027")).toBeInTheDocument()
        expect(screen.getByText("Choose up to three.")).toBeInTheDocument()
        expect(screen.getByTestId("a-contest")).toBeInTheDocument()
    })

    it("carries the class names a client's stylesheet targets, nested as the portal nests them", () => {
        asThePortalCallsIt()

        const page = document.querySelector(".voting-screen.screen")
        expect(page).not.toBeNull()
        expect(page?.querySelector(".stepper-box")).not.toBeNull()

        const heading = page?.querySelector(".title-container")
        expect(heading).not.toBeNull()
        expect(heading?.querySelector(".selected-election-title")?.textContent).toEqual(
            "Board of Directors 2027"
        )
        expect(page?.querySelector(".description")?.textContent).toEqual("Choose up to three.")
    })

    it("frames the breadcrumb 48px under the header", () => {
        asThePortalCallsIt()

        expect(screen.getByTestId("the-breadcrumb").parentElement).toHaveStyle({
            marginTop: "48px",
        })
    })

    it("leaves the description out entirely when the election has none", () => {
        // The portal renders nothing rather than an empty paragraph, and an empty
        // paragraph would take vertical space a voter would see.
        asThePortalCallsIt({description: undefined})

        expect(document.querySelector(".description")).toBeNull()
    })

    it("puts the buttons after the contests", () => {
        asThePortalCallsIt()

        const contest = screen.getByTestId("a-contest")
        const actions = screen.getByTestId("the-actions")

        expect(contest.compareDocumentPosition(actions)).toBe(Node.DOCUMENT_POSITION_FOLLOWING)
    })

    it("puts an adornment beside the name, inside the heading", () => {
        asThePortalCallsIt({titleAdornment: <button type="button">Help</button>})

        expect(document.querySelector(".title-container")?.textContent).toEqual(
            "Board of Directors 2027Help"
        )
    })

    it("invents no wording of its own", () => {
        render(
            <BallotScreenLayout title="A">
                <div />
            </BallotScreenLayout>
        )

        expect(document.body.textContent).toEqual("A")
    })
})
