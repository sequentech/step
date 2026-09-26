// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The row under the confirmation screen: *Print* and *Finish*.
 *
 * Lifted out of the portal's `ConfirmationScreen` for the same reason as `ReviewActions` —
 * the Election Architect's preview drew two plain buttons with the right words and neither
 * shape, and this screen is one a client reviews closely because it is the last thing a
 * voter sees.
 */

import {screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import React from "react"

import {ConfirmationActions} from "./ConfirmationActions"
import {catalogue, inAHost, OTHER_WORDS, PORTAL_WORDS} from "./testCatalogue"

const render = inAHost
const WORDS = PORTAL_WORDS.confirmationScreen

describe("the buttons under a cast ballot", () => {
    it("offers Print and Finish, in the catalogue's words", () => {
        render(<ConfirmationActions />)

        expect(screen.getByText(WORDS.printButton)).toBeInTheDocument()
        expect(screen.getByText(WORDS.finishButton)).toBeInTheDocument()
    })

    it("draws Print as the secondary of the two, with its printer", () => {
        const {container} = render(<ConfirmationActions onPrint={() => undefined} />)

        const print = screen.getByText(WORDS.printButton).closest("button")
        expect(print?.className).toContain("secondary")
        expect(container.querySelectorAll("svg").length).toBeGreaterThanOrEqual(1)
    })

    it("carries the class name a client's stylesheet targets", () => {
        render(<ConfirmationActions onFinish={() => undefined} />)

        expect(document.querySelector(".finish-button")).not.toBeNull()
    })

    it("waits with a spinner while the receipt is being made", () => {
        render(<ConfirmationActions printing onPrint={() => undefined} />)

        expect(document.querySelector(".MuiCircularProgress-root")).not.toBeNull()
        expect(screen.getByText(WORDS.printButton).closest("button")).toBeDisabled()
    })

    it("draws Print dead where there is no receipt to print", () => {
        // A preview's position: the receipt comes from a cast vote.
        render(<ConfirmationActions onFinish={() => undefined} />)

        expect(screen.getByText(WORDS.printButton).closest("button")).toBeDisabled()
        expect(screen.getByText(WORDS.finishButton).closest("button")).not.toBeDisabled()
    })

    it("reports both to the host", async () => {
        const print = jest.fn()
        const finish = jest.fn()
        render(<ConfirmationActions onPrint={print} onFinish={finish} />)

        await userEvent.click(screen.getByText(WORDS.printButton))
        await userEvent.click(screen.getByText(WORDS.finishButton))

        expect(print).toHaveBeenCalled()
        expect(finish).toHaveBeenCalled()
    })

    it("follows the catalogue into another language", () => {
        render(<ConfirmationActions />, catalogue(OTHER_WORDS, "es"))

        expect(screen.getByText("Imprimir")).toBeInTheDocument()
        expect(screen.queryByText(WORDS.printButton)).toBeNull()
    })
})
