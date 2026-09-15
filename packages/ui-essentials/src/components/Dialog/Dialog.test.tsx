/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import Dialog from "./Dialog"
import theme from "../../services/theme"

jest.mock("../LinkBehavior/LinkBehavior", () => "a")
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

it("keeps feature and inner hooks on the portalled dialog and its real controls", () => {
    const handleClose = jest.fn()
    const {container} = render(
        <ThemeProvider theme={theme}>
            <main className="test-screen">
                <Dialog
                    className="receipt-help-dialog"
                    open
                    title="Receipt help"
                    ok="Continue"
                    cancel="Cancel"
                    variant="info"
                    expandable
                    hasCloseButton
                    errorMessage="Receipt unavailable"
                    handleClose={handleClose}
                >
                    <p className="receipt-help-description">Receipt details</p>
                </Dialog>
            </main>
        </ThemeProvider>
    )
    const dialog = screen.getByRole("dialog", {name: "Receipt help"})
    const portal = dialog.closest(".receipt-help-dialog")!
    expect(container.querySelector(".receipt-help-dialog")).toBeNull()
    expect(portal).toHaveClass("dialog")
    expect(dialog).toHaveClass("dialog-paper")
    expect(portal.querySelector(".dialog-backdrop")).toBeInTheDocument()
    expect(portal.querySelector(".dialog-container")).toContainElement(dialog)
    expect(portal.querySelector(".dialog-content")).toHaveTextContent("Receipt details")
    expect(screen.getByRole("alert")).toHaveClass("dialog-error")
    expect(screen.getByRole("button", {name: "Cancel"}).parentElement).toHaveClass("dialog-actions")
    expect(portal.querySelector(".dialog-expand-button")).toHaveProperty("tagName", "BUTTON")
    const close = screen.getByRole("button", {name: "a11y.closeDialog"})
    expect(close).toHaveClass("dialog-close-button")
    expect(close.querySelector("svg")).toHaveClass("dialog-icon-close")
    fireEvent.click(close)
    expect(handleClose).toHaveBeenCalledWith(false)
})
