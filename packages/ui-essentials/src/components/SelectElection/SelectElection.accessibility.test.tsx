/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, render, screen, fireEvent} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material"
import SelectElection from "./SelectElection"
import theme from "../../services/theme"

jest.mock("@sequentech/ui-core", () => ({isUndefined: (value: unknown) => value === undefined}), {
    virtual: true,
})
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("../LinkBehavior/LinkBehavior", () => "a")

it("opens the card once while keeping its nested actions independent", async () => {
    const vote = jest.fn()
    const locate = jest.fn()
    render(
        <ThemeProvider theme={theme}>
            <SelectElection
                title="Council"
                isActive
                isOpen
                isStarted
                hasVoted={false}
                onClickToVote={vote}
                onClickBallotLocator={locate}
                electionHomeUrl="#website"
                resultsUrl="#results"
            />
        </ThemeProvider>
    )
    fireEvent.click(screen.getByRole("heading", {name: "Council"}))
    expect(vote).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByRole("heading", {name: "Council"}).closest(".election-item")!)
    expect(vote).toHaveBeenCalledTimes(2)
    vote.mockClear()
    fireEvent.click(screen.getAllByText("selectElection.electionWebsite")[0])
    expect(vote).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole("link", {name: "selectElection.resultsButton"}))
    expect(vote).not.toHaveBeenCalled()
    const user = userEvent.setup()
    const locateButton = screen.getByRole("button", {
        name: "selectElection.ballotLocator — Council",
    })
    await user.click(locateButton)
    expect(locate).toHaveBeenCalledTimes(1)
    expect(vote).not.toHaveBeenCalled()
    locate.mockClear()
    await act(async () => locateButton.focus())
    await user.keyboard("{Enter}")
    expect(locate).toHaveBeenCalledTimes(1)
    expect(vote).not.toHaveBeenCalled()
    const voteButton = screen.getByRole("button", {name: "selectElection.voteButton — Council"})
    await act(async () => voteButton.focus())
    await user.keyboard(" ")
    expect(vote).toHaveBeenCalledTimes(1)
    await user.keyboard("{Enter}")
    expect(vote).toHaveBeenCalledTimes(2)
    await user.click(voteButton)
    expect(vote).toHaveBeenCalledTimes(3)
})

it("keeps the vote action disabled when no handler is available", async () => {
    const locate = jest.fn()
    render(
        <ThemeProvider theme={theme}>
            <SelectElection
                title="Council"
                isActive={false}
                isOpen
                isStarted
                hasVoted={false}
                onClickBallotLocator={locate}
            />
        </ThemeProvider>
    )
    expect(screen.getByRole("button", {name: "selectElection.voteButton — Council"})).toBeDisabled()
    await userEvent
        .setup()
        .click(screen.getByRole("button", {name: "selectElection.ballotLocator — Council"}))
    expect(locate).toHaveBeenCalledTimes(1)
})

it("does not activate a closed election card even if a vote callback exists", () => {
    const vote = jest.fn()
    render(
        <ThemeProvider theme={theme}>
            <SelectElection
                title="Council"
                isActive={false}
                isOpen={false}
                isStarted
                hasVoted={false}
                onClickToVote={vote}
            />
        </ThemeProvider>
    )
    fireEvent.click(screen.getByRole("heading", {name: "Council"}))
    expect(vote).not.toHaveBeenCalled()
    expect(screen.queryByRole("button", {name: "selectElection.voteButton — Council"})).toBeNull()
})

describe("the vote button of a ballot already cast", () => {
    const renderCard = (props: Partial<React.ComponentProps<typeof SelectElection>> = {}) =>
        render(
            <ThemeProvider theme={theme}>
                <SelectElection
                    title="Council"
                    isActive={false}
                    isOpen
                    isStarted
                    hasVoted
                    {...props}
                />
            </ThemeProvider>
        )
    const voteButton = () =>
        screen.getByRole("button", {name: "selectElection.voteButton — Council"})

    it("says why it is off, to a pointer and to assistive technology", () => {
        renderCard({alreadyCast: true})
        expect(voteButton()).toBeDisabled()
        // A disabled button gets no pointer events, so the element around it carries the title.
        expect(voteButton().closest("[title]")).toHaveAttribute(
            "title",
            "selectElection.alreadyCast"
        )
        expect(voteButton()).toHaveAccessibleDescription("selectElection.alreadyCast")
    })

    it("stays quiet when the card does not say the ballot was cast", () => {
        renderCard()
        expect(voteButton()).toBeDisabled()
        expect(voteButton().closest("[title]")).toBeNull()
        expect(voteButton()).toHaveAccessibleDescription("")
    })

    it("stays quiet while the button can be pressed", () => {
        renderCard({alreadyCast: true, isActive: true, onClickToVote: jest.fn()})
        expect(voteButton()).toBeEnabled()
        expect(voteButton().closest("[title]")).toBeNull()
        expect(voteButton()).toHaveAccessibleDescription("")
    })

    it("has no button, and no hint, when the election is closed", () => {
        renderCard({alreadyCast: true, isOpen: false})
        expect(screen.queryByRole("button", {name: /voteButton/})).toBeNull()
        expect(screen.queryByText("selectElection.alreadyCast")).toBeNull()
    })
})
