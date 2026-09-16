/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen, cleanup} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import Dialog from "./Dialog"
import theme from "../../services/theme"

jest.mock("../LinkBehavior/LinkBehavior", () => "a")
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

afterEach(() => {
    cleanup()
    jest.restoreAllMocks()
})

it("makes background controls inert only while open and restores trigger focus", () => {
    const Example = () => {
        const [open, setOpen] = React.useState(false)
        return (
            <ThemeProvider theme={theme}>
                <button onClick={() => setOpen(true)}>Open help</button>
                <Dialog
                    open={open}
                    title="Help"
                    cancel="Cancel"
                    handleClose={() => setOpen(false)}
                />
            </ThemeProvider>
        )
    }
    const {container} = render(<Example />)
    const trigger = screen.getByRole("button", {name: "Open help"})
    trigger.focus()
    fireEvent.click(trigger)
    expect(container).toHaveAttribute("aria-hidden", "true")
    expect(container).toHaveAttribute("inert")
    expect(screen.getByRole("dialog").closest("[inert]")).toBeNull()
    fireEvent.click(screen.getByRole("button", {name: "Cancel"}))
    expect(container).not.toHaveAttribute("inert")
    expect(trigger).toHaveFocus()
    fireEvent.click(trigger)
    expect(container).toHaveAttribute("inert")
    fireEvent.keyDown(screen.getByRole("dialog"), {key: "Escape"})
    expect(container).not.toHaveAttribute("inert")
})

it("keeps the page inert when overlapping dialogs close in either order", () => {
    const dialogs = (first: boolean, second: boolean) => (
        <ThemeProvider theme={theme}>
            <Dialog open={first} title="First" cancel="Cancel first" handleClose={jest.fn()} />
            <Dialog open={second} title="Second" cancel="Cancel second" handleClose={jest.fn()} />
        </ThemeProvider>
    )
    const {container, rerender, unmount} = render(dialogs(true, false))
    const firstRoot = screen.getByRole("dialog", {name: "First"}).closest(".dialog")!
    rerender(dialogs(true, true))
    expect(container).toHaveAttribute("inert")
    expect(firstRoot).toHaveAttribute("inert")
    expect(screen.getByRole("dialog", {name: "Second"}).closest("[inert]")).toBeNull()
    rerender(dialogs(true, false))
    expect(container).toHaveAttribute("inert")
    expect(firstRoot).not.toHaveAttribute("inert")
    rerender(dialogs(true, true))
    rerender(dialogs(false, true))
    expect(container).toHaveAttribute("inert")
    unmount()
    expect(container).not.toHaveAttribute("inert")
})

it("preserves pre-existing inert backgrounds and leaves closed popovers available", () => {
    const background = document.createElement("section")
    background.setAttribute("inert", "")
    const closedPopover = document.createElement("div")
    closedPopover.className = "MuiModal-root MuiModal-hidden"
    document.body.append(background, closedPopover)
    try {
        const {unmount} = render(
            <ThemeProvider theme={theme}>
                <Dialog open title="Help" cancel="Cancel" handleClose={jest.fn()} />
            </ThemeProvider>
        )
        expect(background).toHaveAttribute("inert")
        expect(closedPopover).not.toHaveAttribute("inert")
        unmount()
        expect(background).toHaveAttribute("inert")
    } finally {
        background.remove()
        closedPopover.remove()
    }
})

it("does not inert the top dialog when two dialogs mount together", () => {
    const {container} = render(
        <ThemeProvider theme={theme}>
            <Dialog open title="First" cancel="Cancel first" handleClose={jest.fn()} />
            <Dialog open title="Second" cancel="Cancel second" handleClose={jest.fn()} />
        </ThemeProvider>
    )
    expect(container).toHaveAttribute("inert")
    expect(screen.getByRole("dialog", {name: "Second"}).closest("[inert]")).toBeNull()
})

it.each(["Vote has not been cast", "Do you want to audit the ballot?"])(
    "%s wraps keyboard focus without tabbable non-widget guards",
    async (title) => {
        jest.spyOn(HTMLElement.prototype, "getClientRects").mockImplementation(function (
            this: HTMLElement
        ) {
            return (this.closest("[hidden]")
                ? []
                : [new DOMRect(0, 0, 100, 30)]) as unknown as DOMRectList
        })
        render(
            <ThemeProvider theme={theme}>
                <Dialog open title={title} cancel="Cancel" ok="Audit" handleClose={jest.fn()}>
                    <button className="disabled-control" disabled>
                        Disabled
                    </button>
                    <button className="hidden-control" hidden>
                        Hidden
                    </button>
                </Dialog>
            </ThemeProvider>
        )
        const dialog = screen.getByRole("dialog", {name: title})
        const portal = dialog.closest(".dialog")!
        for (const guard of Array.from(portal.querySelectorAll('[data-testid^="sentinel"]'))) {
            expect(getComputedStyle(guard).display).toBe("none")
        }
        const cancel = screen.getByRole("button", {name: "Cancel"})
        const audit = screen.getByRole("button", {name: "Audit"})
        await act(async () => {
            cancel.focus()
        })
        await act(async () => {
            fireEvent.keyDown(cancel, {key: "Tab", shiftKey: true})
        })
        expect(audit).toHaveFocus()
        await act(async () => {
            fireEvent.keyDown(audit, {key: "Tab"})
        })
        expect(cancel).toHaveFocus()
        await act(async () => {
            fireEvent.keyDown(cancel, {key: "Tab"})
        })
        expect(cancel).toHaveFocus() // Interior Tab is left to the browser.
    }
)

it("keeps focus in a dialog with no enabled controls", () => {
    render(
        <ThemeProvider theme={theme}>
            <Dialog
                open
                title="Working"
                ok="Continue"
                okEnabled={() => false}
                handleClose={jest.fn()}
            />
        </ThemeProvider>
    )
    const dialog = screen.getByRole("dialog", {name: "Working"})
    const container = dialog.closest(".dialog-container")!
    fireEvent.keyDown(container, {key: "Tab"})
    expect(dialog).toHaveFocus()
})

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
