/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import theme from "../../services/theme"
import {PreferentialCandidateResults} from "./PreferentialCandidateResults"
import type {PreferentialProcessResults, PreferentialRound} from "./types"

jest.mock(
    "@sequentech/ui-core",
    () => ({
        ...jest.requireActual<typeof import("../../../../ui-core/src/types/VotingChannel")>(
            "../../../../ui-core/src/types/VotingChannel"
        ),
        formatPercentOne: (value: number) => `${value}%`,
    }),
    {virtual: true}
)

const ALICE = {id: "alice/one", name: "Alice"}
const BOB = {id: "bob", name: "Bob"}
const PROCESS_ONLY = {id: "process-only", name: "Process-only candidate"}
function round(wins: number, options: Partial<PreferentialRound> = {}): PreferentialRound {
    return {
        winner: null,
        eliminated_candidates: null,
        active_candidates_count: 2,
        active_ballots_count: 10,
        exhausted_ballots_count: 0,
        candidates_wins: {
            [ALICE.id]: {name: ALICE.name, wins, transference: 0, percentage: wins / 10},
            [BOB.id]: {name: BOB.name, wins: 10 - wins, transference: 0, percentage: 1 - wins / 10},
        },
        ...options,
    }
}
const PROCESS: PreferentialProcessResults = {
    name_references: [BOB, PROCESS_ONLY, ALICE],
    round_count: 3,
    max_rounds: 3,
    rounds: [round(4, {eliminated_candidates: [BOB]}), round(6), round(10, {winner: ALICE})],
}
function content(processResults = PROCESS) {
    return (
        <ThemeProvider theme={theme}>
            <PreferentialCandidateResults
                processResults={processResults}
                candidates={[ALICE, BOB]}
                labels={{
                    round: "Round",
                    nextRounds: "Next rounds",
                    previousRounds: "Previous rounds",
                    winner: "Winner",
                    eliminated: "Eliminated",
                }}
            />
        </ThemeProvider>
    )
}

it("preserves configured candidate order and carries elimination state into later rounds", async () => {
    const user = userEvent.setup()
    const {container} = render(content())
    expect(screen.getAllByRole("rowheader").map((cell) => cell.textContent)).toEqual([
        "Alice",
        "Bob",
        "Process-only candidate",
    ])
    const bob = screen.getByRole("rowheader", {name: "Bob"}).closest("tr")!
    expect(within(bob).getByText("Eliminated")).toBeVisible()
    expect(within(bob).getByText("6 (60.00%)")).toBeVisible()
    await user.click(screen.getByRole("button", {name: "Next rounds"}))
    expect(screen.getByText("Round 2")).toBeVisible()
    expect(within(bob).getByText("Eliminated")).toBeVisible()
    await user.click(screen.getByRole("button", {name: "Next rounds"}))
    expect(screen.getByText("Winner")).toBeVisible()
    expect(screen.queryByRole("button", {name: "Next rounds"})).toBeNull()
    expect(
        container.querySelector(".seq-tally-results-preferential-results__row--candidate-alice-one")
    ).not.toBeNull()
    await user.click(screen.getByRole("button", {name: "Previous rounds"}))
    expect(screen.getByText("Round 2")).toBeVisible()
})

it("bounds keyboard navigation and resets the window when the process changes", () => {
    const {container, rerender} = render(content())
    const table = container.querySelector<HTMLElement>(".seq-tally-results-preferential-results")!
    fireEvent.keyDown(table, {key: "ArrowLeft"})
    expect(screen.getByText("Round 1")).toBeVisible()
    fireEvent.keyDown(table, {key: "End"})
    expect(screen.getByText("Round 3")).toBeVisible()
    fireEvent.keyDown(table, {key: "ArrowRight"})
    expect(screen.getByText("Round 3")).toBeVisible()
    fireEvent.keyDown(table, {key: "ArrowLeft"})
    expect(screen.getByText("Round 2")).toBeVisible()
    fireEvent.keyDown(table, {key: "Home"})
    expect(screen.getByText("Round 1")).toBeVisible()
    fireEvent.keyDown(table, {key: "ArrowRight"})
    fireEvent.keyDown(table, {key: "Tab"})
    expect(screen.getByText("Round 2")).toBeVisible()
    rerender(content({...PROCESS, rounds: [PROCESS.rounds[0]], round_count: 1}))
    expect(screen.getByText("Round 1")).toBeVisible()
    expect(screen.queryByRole("button")).toBeNull()
})

it("does not invent a round when the tally has not produced any rounds", () => {
    render(content({...PROCESS, rounds: [], round_count: 0}))
    expect(screen.queryByRole("table")).toBeNull()
})
