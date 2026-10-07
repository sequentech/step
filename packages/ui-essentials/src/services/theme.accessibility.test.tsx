/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {Checkbox, ThemeProvider, getContrastRatio} from "@mui/material"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import IconButton from "../components/IconButton/IconButton"
import theme from "./theme"

jest.mock("../components/LinkBehavior/LinkBehavior", () => "a")

const focusWithKeyboard = async (element: HTMLElement) => {
    fireEvent.keyDown(document, {key: "Tab"})
    await act(async () => element.focus())
}

describe("shared control contrast", () => {
    it.each([false, true])("keeps an enabled checkbox discernible (checked: %s)", (checked) => {
        render(
            <ThemeProvider theme={theme}>
                <Checkbox checked={checked} slotProps={{input: {"aria-label": "Declaration"}}} />
            </ThemeProvider>
        )
        const control = screen.getByRole("checkbox").closest(".MuiCheckbox-root")!
        const color = getComputedStyle(control).color

        for (const background of ["#ffffff", theme.palette.lightBackground]) {
            expect(getContrastRatio(color, background)).toBeGreaterThanOrEqual(3)
        }
    })

    it("keeps a persistent two-tone focus ring while a checkbox toggles", async () => {
        render(
            <ThemeProvider theme={theme}>
                <Checkbox slotProps={{input: {"aria-label": "Declaration"}}} disableRipple />
            </ThemeProvider>
        )
        const checkbox = screen.getByRole("checkbox")
        const control = checkbox.closest(".MuiCheckbox-root")!
        await focusWithKeyboard(checkbox)

        expect(control).toHaveClass("Mui-focusVisible")
        expect(control).toHaveStyle({
            outline: "2px solid black",
            outlineOffset: "2px",
            boxShadow: "0 0 0 2px white",
        })
        fireEvent.click(checkbox)
        expect(checkbox).toBeChecked()
        expect(control).toHaveStyle({outline: "2px solid black", boxShadow: "0 0 0 2px white"})
    })

    it("gives the shared help button a focus ring independent of its icon color", async () => {
        render(
            <ThemeProvider theme={theme}>
                <IconButton icon={faCircleQuestion} ariaLabel="Help" sx={{color: "#b8c0cc"}} />
            </ThemeProvider>
        )
        const help = screen.getByRole("button", {name: "Help"})
        await focusWithKeyboard(help)

        expect(help).toHaveClass("Mui-focusVisible")
        expect(help).toHaveStyle({
            outline: "2px solid black",
            outlineOffset: "2px",
            boxShadow: "0 0 0 2px white",
        })
    })
})
