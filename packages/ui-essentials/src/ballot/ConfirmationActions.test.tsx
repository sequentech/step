// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The row under *Keep your receipt*: *Save receipt (PDF)* and *Finish* (design v42).
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
    it("offers Save receipt and Finish, in the catalogue's words", () => {
        render(<ConfirmationActions />)

        expect(screen.getByText(WORDS.saveReceiptButton)).toBeInTheDocument()
        expect(screen.getByText(WORDS.finishButton)).toBeInTheDocument()
    })

    it("draws Save receipt as the outlined one, with a download arrow, and Finish filled", () => {
        render(<ConfirmationActions onPrint={() => undefined} onFinish={() => undefined} />)

        const save = screen.getByText(WORDS.saveReceiptButton).closest("button")
        expect(save?.className).toContain("secondary")
        expect(save?.querySelector("svg.print-receipt-icon")).not.toBeNull()
        const finish = screen.getByText(WORDS.finishButton).closest("button")
        expect(finish?.className).not.toContain("secondary")
    })

    it("puts Finish at the end of the same row as Save receipt", () => {
        const {container} = render(
            <ConfirmationActions onPrint={() => undefined} onFinish={() => undefined} />
        )

        const row = container.querySelector(".actions-container")!
        const save = row.querySelector(".save-receipt-button")!
        const finish = row.querySelector(".finish-button")!
        expect(save.parentElement).toBe(row)
        expect(finish.parentElement).toBe(row)
        expect(save.compareDocumentPosition(finish)).toBe(Node.DOCUMENT_POSITION_FOLLOWING)
        expect(row.lastElementChild).toBe(finish)
        expect(getComputedStyle(finish).marginLeft).toBe("auto")
    })

    it("gives neither button a fixed width, so a long translation wraps", () => {
        render(<ConfirmationActions onPrint={() => undefined} onFinish={() => undefined} />)

        for (const button of Array.from(document.querySelectorAll("button"))) {
            expect(getComputedStyle(button).width).toBe("")
        }
    })

    it("says a copy was emailed only when told so", () => {
        const quiet = render(<ConfirmationActions onPrint={() => undefined} />)
        expect(document.querySelector(".cast-email-note")).toBeNull()
        quiet.unmount()

        render(<ConfirmationActions onPrint={() => undefined} castEmailSent />)
        expect(document.querySelector(".cast-email-note")).toHaveTextContent(
            "confirmationScreen.castEmailNote"
        )
    })

    it("carries the class name a client's stylesheet targets", () => {
        render(<ConfirmationActions onFinish={() => undefined} />)

        expect(document.querySelector(".finish-button")).not.toBeNull()
    })

    it("waits with a spinner while the receipt is being made", () => {
        render(<ConfirmationActions printing onPrint={() => undefined} />)

        expect(document.querySelector(".MuiCircularProgress-root")).not.toBeNull()
        expect(screen.getByText(WORDS.saveReceiptButton).closest("button")).toBeDisabled()
    })

    it("draws Save receipt dead where there is no receipt to save", () => {
        // A preview's position: the receipt comes from a cast vote.
        render(<ConfirmationActions onFinish={() => undefined} />)

        expect(screen.getByText(WORDS.saveReceiptButton).closest("button")).toBeDisabled()
        expect(screen.getByText(WORDS.finishButton).closest("button")).not.toBeDisabled()
    })

    it("reports both to the host", async () => {
        const print = jest.fn()
        const finish = jest.fn()
        render(<ConfirmationActions onPrint={print} onFinish={finish} />)

        await userEvent.click(screen.getByText(WORDS.saveReceiptButton))
        await userEvent.click(screen.getByText(WORDS.finishButton))

        expect(print).toHaveBeenCalled()
        expect(finish).toHaveBeenCalled()
    })

    it("follows the catalogue into another language", () => {
        render(<ConfirmationActions />, catalogue(OTHER_WORDS, "es"))

        expect(screen.getByText("Guardar recibo")).toBeInTheDocument()
        expect(screen.queryByText(WORDS.saveReceiptButton)).toBeNull()
    })
})

describe("the ovcs fixes carried into the shared confirmation row", () => {
    it("leaves Save receipt out when there is nothing to save", () => {
        render(<ConfirmationActions withPrint={false} onFinish={() => undefined} />)

        expect(document.querySelector(".print-receipt-button")).toBeNull()
        expect(document.querySelector(".finish-button")).not.toBeNull()
    })

    it("announces the wait for a receipt rather than only spinning", () => {
        const {rerender} = render(<ConfirmationActions printing onPrint={() => undefined} />)

        expect(screen.getByRole("status")).toHaveTextContent("a11y.loading")
        expect(document.querySelector(".print-receipt-progress")).toHaveAttribute(
            "aria-hidden",
            "true"
        )

        rerender(<ConfirmationActions onPrint={() => undefined} />)
        expect(screen.getByRole("status")).toHaveTextContent("")
    })
})
