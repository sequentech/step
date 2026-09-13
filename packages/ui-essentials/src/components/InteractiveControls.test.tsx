/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "node:assert/strict"
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import theme from "../services/theme"
import CandidatesList from "./CandidatesList/CandidatesList"
import ExpandableText from "./ExpandableText/ExpandableText"
import LogoutButton from "./LogoutButton/LogoutButton"
import Dialog from "./Dialog/Dialog"
import SelectElection from "./SelectElection/SelectElection"

jest.mock("@sequentech/ui-core", () => ({isUndefined: (value: unknown) => value === undefined}), {
    virtual: true,
})

jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
const withTheme = (content: React.ReactNode) => (
    <ThemeProvider theme={theme}>{content}</ThemeProvider>
)

it("expands long text with the keyboard and restores its preview without interpreting HTML", async () => {
    const user = userEvent.setup()
    render(
        withTheme(
            <ExpandableText
                text="<script>plain text only</script>"
                initialLength={8}
                showMoreLabel="Read more"
                showLessLabel="Read less"
                preformatted
            />
        )
    )
    const toggle = screen.getByRole("button", {name: "Read more"})
    toggle.focus()
    await user.keyboard("{Enter}")
    expect(screen.getByText("<script>plain text only</script>")).toBeVisible()
    expect(document.querySelector("script")).toBeNull()
    await user.click(screen.getByRole("button", {name: "Read less"}))
    expect(screen.queryByText("<script>plain text only</script>")).toBeNull()
})

it("does not add expansion controls for text that already fits", () => {
    render(
        withTheme(
            <ExpandableText
                text="Brief"
                initialLength={5}
                showMoreLabel="More"
                showLessLabel="Less"
            />
        )
    )
    expect(screen.getByText("Brief")).toBeVisible()
    expect(screen.queryByRole("button")).toBeNull()
})

it("keeps category expansion separate from checking the whole list", async () => {
    const user = userEvent.setup()
    const setChecked = jest.fn()
    const onExpandedChange = jest.fn()
    const {rerender} = render(
        withTheme(
            <CandidatesList
                title="Council"
                isActive
                isCheckable
                checked={false}
                setChecked={setChecked}
                isCollapsible
                defaultExpanded={false}
                onExpandedChange={onExpandedChange}
                collapseToggleAriaLabel="Show council"
                titleComponent="h3"
            >
                <li>Candidate</li>
            </CandidatesList>
        )
    )
    await user.click(screen.getByRole("button", {name: "Show council"}))
    expect(onExpandedChange).toHaveBeenCalledWith(true)
    expect(setChecked).not.toHaveBeenCalled()
    await user.click(screen.getByRole("checkbox"))
    expect(setChecked).toHaveBeenCalledTimes(1)
    expect(setChecked).toHaveBeenLastCalledWith(true)
    rerender(
        withTheme(
            <CandidatesList
                title="Council"
                isCollapsible
                externalExpanded={false}
                collapseToggleAriaLabel="Show council"
            >
                <li>Candidate</li>
            </CandidatesList>
        )
    )
    await waitFor(() =>
        expect(screen.getByRole("button")).toHaveAttribute("aria-expanded", "false")
    )
})

it.each([{isActive: false}, {shouldDisable: true}, {isCheckable: false}])(
    "does not select a list when its selection policy forbids it (%j)",
    async (policy) => {
        const user = userEvent.setup()
        const setChecked = jest.fn()
        render(
            withTheme(
                <CandidatesList
                    title="Council"
                    isActive
                    isCheckable
                    setChecked={setChecked}
                    {...policy}
                >
                    <li>Candidate</li>
                </CandidatesList>
            )
        )
        await user.click(screen.getByText("Council"))
        const checkbox = screen.queryByRole("checkbox")
        if (checkbox) {
            expect(checkbox).toBeDisabled()
            // A disabled control cannot receive a pointer event from a user.
            fireEvent.click(checkbox)
        }
        expect(setChecked).not.toHaveBeenCalled()
    }
)

it("requires confirmation before logout and lets the voter cancel", async () => {
    const user = userEvent.setup()
    const logoutFn = jest.fn()
    render(withTheme(<LogoutButton logoutFn={logoutFn} />))
    await user.click(screen.getByRole("button", {name: "logout.buttonText"}))
    expect(screen.getByRole("dialog", {name: "logout.modal.title"})).toBeVisible()
    await user.click(screen.getByRole("button", {name: "logout.modal.close"}))
    expect(logoutFn).not.toHaveBeenCalled()
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
    await user.click(screen.getByRole("button", {name: "logout.buttonText"}))
    await user.click(screen.getByRole("button", {name: "logout.modal.ok"}))
    expect(logoutFn).toHaveBeenCalledTimes(1)
})

it("keeps a disabled confirmation inactive while allowing escape cancellation", async () => {
    const user = userEvent.setup()
    const handleClose = jest.fn()
    render(
        withTheme(
            <Dialog
                open
                title="Confirm election"
                ok="Confirm"
                cancel="Cancel"
                handleClose={handleClose}
                okEnabled={() => false}
                errorMessage="Select an election first"
                variant="info"
            >
                Details
            </Dialog>
        )
    )
    expect(screen.getByRole("dialog")).toHaveAccessibleDescription("Select an election first")
    expect(screen.getByRole("button", {name: "Confirm"})).toBeDisabled()
    fireEvent.click(screen.getByRole("button", {name: "Confirm"}))
    expect(handleClose).not.toHaveBeenCalled()
    fireEvent.keyDown(screen.getByRole("dialog"), {key: "Escape", code: "Escape"})
    expect(handleClose).toHaveBeenCalledWith(false)
})

it("removes the election countdown at its deadline without leaving a stray zero", () => {
    jest.useFakeTimers()
    const now = new Date("2026-01-15T12:00:00Z")
    jest.setSystemTime(now)
    try {
        const {container} = render(
            withTheme(
                <SelectElection
                    title="Council"
                    isActive
                    isOpen={false}
                    hasVoted={false}
                    isStarted={false}
                    electionDates={{
                        first_started_at: new Date(now.getTime() + 2_000).toISOString(),
                    }}
                />
            )
        )
        expect(container.querySelector(".election-countdown")).not.toBeNull()
        act(() => jest.advanceTimersByTime(2_000))
        expect(container.querySelector(".election-countdown")).toBeNull()
        const electionItem = container.querySelector(".election-list-item")
        assert.ok(electionItem, "Starting an election must preserve its list item")
        const visibleText = Array.from(electionItem.childNodes)
            .filter((node) => node.nodeType === Node.TEXT_NODE)
            .map((node) => node.textContent)
        expect(visibleText).toEqual([])
    } finally {
        jest.useRealTimers()
    }
})
