// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The ballot list screen's arrangement, and the class names on it.
 *
 * Lifted out of the portal's `ElectionSelectionScreen` so the Election Architect's
 * Ballot Preview draws this screen rather than a second one that looks like it. The
 * usual reason for these tests applies — an arrangement shared by two applications is
 * a contract — but this layout has a second one: a client's stylesheet targets
 * `election-selection-screen`, `title-section`, `election-selection-heading` and
 * `elections-list` by name, and targets them *nested that way*. Renaming or reparenting
 * any of them silently unstyles a deployed election, which is why the shape is
 * asserted here and not only the words.
 *
 * The wording is a prop, as in `StartLayout`: `electionSelectionScreen.*` lives in
 * *voting-portal*'s catalogue, so a layout that translated for itself would draw raw
 * keys in the wizard.
 */

import {screen} from "@testing-library/react"
import React from "react"

import {ElectionListLayout} from "./ElectionListLayout"
import {catalogue, inAHost, OTHER_WORDS, PORTAL_WORDS} from "./testCatalogue"

const render = inAHost
const WORDS = PORTAL_WORDS.electionSelectionScreen

/** The layout as the portal calls it, less the parts each test varies. */
const asThePortalCallsIt = (props: Partial<React.ComponentProps<typeof ElectionListLayout>> = {}) =>
    render(
        <ElectionListLayout steps={<div data-testid="the-breadcrumb" />} {...props}>
            <div data-testid="an-election" />
        </ElectionListLayout>
    )

describe("the screen that asks which ballot to vote", () => {
    it("shows the heading, the description and the elections", () => {
        asThePortalCallsIt()

        expect(screen.getByText(WORDS.title)).toBeInTheDocument()
        expect(screen.getByText(WORDS.description)).toBeInTheDocument()
        expect(screen.getByTestId("an-election")).toBeInTheDocument()
    })

    it("carries the four class names a client's stylesheet targets, nested as the portal nests them", () => {
        asThePortalCallsIt()

        const page = document.querySelector(".election-selection-screen.screen")
        expect(page).not.toBeNull()

        const section = page?.querySelector(".title-section")
        expect(section).not.toBeNull()
        expect(section?.querySelector(".election-selection-heading")).not.toBeNull()
        expect(section?.querySelector(".election-event-actions")).not.toBeNull()
        expect(page?.querySelector(".elections-list")).not.toBeNull()
    })

    it("puts the breadcrumb above the heading, not below it", () => {
        asThePortalCallsIt()

        const page = document.querySelector(".election-selection-screen") as HTMLElement
        const breadcrumb = screen.getByTestId("the-breadcrumb")
        const section = page.querySelector(".title-section") as HTMLElement

        expect(breadcrumb.compareDocumentPosition(section)).toBe(Node.DOCUMENT_POSITION_FOLLOWING)
    })

    it("frames the breadcrumb 48px under the header, which the caller does not have to know", () => {
        // The measurement the wizard's preview got wrong: it framed the stepper
        // itself, 16px above the screen instead of 48px inside it.
        asThePortalCallsIt()

        const framed = screen.getByTestId("the-breadcrumb").parentElement
        expect(framed).toHaveStyle({marginTop: "48px"})
    })

    it("leaves the breadcrumb out when there is none to draw", () => {
        asThePortalCallsIt({steps: undefined})

        expect(screen.queryByTestId("the-breadcrumb")).toBeNull()
        // ...and the heading is still there, i.e. nothing else depended on it.
        expect(screen.getByText(WORDS.title)).toBeInTheDocument()
    })

    it("keeps the actions container even when there are no actions", () => {
        // The portal renders `PageActions` unconditionally and its buttons
        // conditionally. A preview with no buttons has to keep the same tree.
        asThePortalCallsIt()

        expect(document.querySelector(".election-event-actions")).not.toBeNull()
    })

    it("puts the actions in that container when there are some", () => {
        asThePortalCallsIt({actions: <button type="button">Results</button>})

        expect(document.querySelector(".election-event-actions")?.textContent).toEqual("Results")
    })

    it("puts an adornment inside the heading, beside the title", () => {
        // The portal's help button and its dialog go here.
        asThePortalCallsIt({titleAdornment: <button type="button">Help</button>})

        const heading = document.querySelector(".election-selection-heading h1")
        expect(heading?.textContent).toEqual(`${WORDS.title}Help`)
    })

    it("shows a warning instead of the description rather than as well as it", () => {
        asThePortalCallsIt({alert: <div role="alert">This election has closed</div>})

        expect(screen.getByRole("alert")).toBeInTheDocument()
        expect(screen.queryByText(WORDS.description)).toBeNull()
    })

    it("takes its two strings from the catalogue and adds nothing", () => {
        render(
            <ElectionListLayout>
                <div />
            </ElectionListLayout>
        )

        // `electionSelectionScreen.title` and `.description`, and nothing else: no
        // English of its own, which is what `EA-F2-053` removed from this file.
        expect(document.body.textContent).toEqual(`${WORDS.title}${WORDS.description}`)
    })

    it("follows the catalogue into another language", () => {
        render(
            <ElectionListLayout>
                <div />
            </ElectionListLayout>,
            catalogue(OTHER_WORDS, "es")
        )

        expect(screen.getByText("Qué papeleta")).toBeInTheDocument()
        expect(screen.queryByText(WORDS.title)).toBeNull()
    })
})
