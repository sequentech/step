/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {Button, MenuItem, Tab, TableSortLabel, ThemeProvider} from "@mui/material"
import ExpandableText from "../components/ExpandableText/ExpandableText"
import CandidatesList from "../components/CandidatesList/CandidatesList"
import LanguageMenu from "../components/LanguageMenu/LanguageMenu"
import theme, {adminTheme} from "./theme"

jest.mock("../components/LinkBehavior/LinkBehavior", () => "a")
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))

const keyboardFocus = async (control: HTMLElement) => {
    fireEvent.keyDown(document, {key: "Tab"})
    await act(async () => control.focus())
    expect(control).toHaveClass("Mui-focusVisible")
}

it.each([
    {portal: "voting", activeTheme: theme},
    {portal: "admin", activeTheme: adminTheme},
])("keeps $portal secondary action keyboard focus visible", async ({activeTheme}) => {
    render(
        <ThemeProvider theme={activeTheme}>
            <Button variant="secondary">Continue</Button>
        </ThemeProvider>
    )
    const control = screen.getByRole("button", {name: "Continue"})
    await keyboardFocus(control)
    expect(control).toHaveStyle({
        outline: "2px solid black",
        outlineOffset: "2px",
        boxShadow: "0 0 0 2px white",
    })
})

// JSDOM does not apply the theme focus selector over the component box-shadow
// resets, so these tests assert the outline and the interaction state only.
it("keeps the ballot locator Show more control visibly focused after activation", async () => {
    render(
        <ThemeProvider theme={theme}>
            <ExpandableText
                text="abcdef"
                initialLength={3}
                showMoreLabel="Show more"
                showLessLabel="Show less"
            />
        </ThemeProvider>
    )
    const control = screen.getByRole("button", {name: "Show more"})
    await keyboardFocus(control)
    expect(control).toHaveStyle({
        outline: "2px solid black",
        outlineOffset: "2px",
    })
    fireEvent.click(control)
    expect(control).toHaveAccessibleName("Show less")
    expect(control).toHaveFocus()
    expect(control).toHaveStyle({
        outline: "2px solid black",
        outlineOffset: "2px",
    })
})

it("keeps the actual candidate list toggle visibly focused through collapse and expansion", async () => {
    const selectList = jest.fn()
    render(
        <ThemeProvider theme={theme}>
            <CandidatesList
                title="Candidate list"
                isActive
                isCheckable
                checked={false}
                setChecked={selectList}
                isCollapsible
                defaultExpanded
                showCandidatesLabel="Show candidates"
                hideCandidatesLabel="Hide candidates"
            >
                <li>Candidate</li>
            </CandidatesList>
        </ThemeProvider>
    )
    const control = screen.getByRole("button", {name: "Hide candidates"})
    await keyboardFocus(control)
    for (let index = 0; index < 3; index++) {
        const expanded = index !== 1
        expect(control).toHaveFocus()
        expect(control).toHaveAttribute("aria-expanded", String(expanded))
        expect(control).toHaveStyle({
            outline: "2px solid black",
            outlineOffset: "2px",
        })
        if (index < 2) {
            fireEvent.click(control)
        }
    }
    expect(selectList).not.toHaveBeenCalled()
})

it.each([
    {name: "locator tab", control: <Tab label="Ballot locator" />, role: "tab"},
    {name: "log sort", control: <TableSortLabel>Ballot ID</TableSortLabel>, role: "button"},
    {name: "language option", control: <MenuItem>English</MenuItem>, role: "menuitem"},
])("keeps $name keyboard focus visible without relying on a ripple", async ({control, role}) => {
    render(<ThemeProvider theme={theme}>{control}</ThemeProvider>)
    const element = screen.getByRole(role)
    await keyboardFocus(element)
    expect(element).toHaveStyle({
        outline: "2px solid black",
        outlineOffset: "-4px",
        boxShadow: "inset 0 0 0 2px white",
    })
})

it("names the language menu when its responsive text is hidden", () => {
    render(
        <ThemeProvider theme={theme}>
            <LanguageMenu />
        </ThemeProvider>
    )
    const label = screen.getByText("language")
    label.style.display = "none"
    expect(screen.getByRole("button")).toHaveAccessibleName("language")
})
