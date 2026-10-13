// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {ThemeProvider} from "@mui/material/styles"
import {render, screen, waitFor, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {theme} from "@sequentech/ui-essentials"
import {I18nextProvider} from "react-i18next"

import {i18n} from "../../testing/ballotHarness"
import {SlateDialog} from "./SlateDialog"

const Opener: React.FC<{onClose: (confirmed: boolean) => void}> = ({onClose}) => {
    const [open, setOpen] = useState(false)
    return (
        <>
            <button onClick={() => setOpen(true)}>Open</button>
            <SlateDialog
                className="test-dialog"
                open={open}
                title="Forward Together"
                ok="Select this slate"
                cancel="Back"
                handleClose={(confirmed) => {
                    setOpen(false)
                    onClose(confirmed)
                }}
            >
                <p>Body</p>
            </SlateDialog>
        </>
    )
}

const mount = () => {
    const onClose = jest.fn()
    render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Opener onClose={onClose} />
            </ThemeProvider>
        </I18nextProvider>
    )
    return onClose
}

const open = async () => {
    await userEvent.click(screen.getByRole("button", {name: "Open"}))
    return screen.getByRole("dialog", {name: "Forward Together"})
}

describe("SlateDialog", () => {
    it("is a dialog named by its h2 title, with its class, body and answers", async () => {
        mount()
        const dialog = await open()

        expect(within(dialog).getByRole("heading", {level: 2})).toHaveTextContent(
            "Forward Together"
        )
        expect(document.querySelector(".test-dialog")).toContainElement(dialog)
        expect(within(dialog).getByText("Body")).toBeVisible()
        const buttons = within(dialog)
            .getAllByRole("button")
            .map((b) => b.textContent)
        expect(buttons).toEqual(["", "Back", "Select this slate"])
    })

    it("moves the focus into the dialog", async () => {
        mount()
        const dialog = await open()

        await waitFor(() => expect(dialog.contains(document.activeElement)).toBe(true))
    })

    it("answers true for the primary action", async () => {
        const onClose = mount()
        await open()

        await userEvent.click(screen.getByRole("button", {name: "Select this slate"}))

        expect(onClose).toHaveBeenCalledWith(true)
    })

    it.each([
        ["Back", async () => userEvent.click(screen.getByRole("button", {name: "Back"}))],
        [
            "the close button",
            async () => userEvent.click(screen.getByRole("button", {name: "Close dialog"})),
        ],
        ["Escape", async () => userEvent.keyboard("{Escape}")],
    ])("answers false for %s and gives the focus back", async (_name, close) => {
        const onClose = mount()
        await open()

        await close()

        expect(onClose).toHaveBeenCalledWith(false)
        await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
        await waitFor(() => expect(screen.getByRole("button", {name: "Open"})).toHaveFocus())
    })
})
