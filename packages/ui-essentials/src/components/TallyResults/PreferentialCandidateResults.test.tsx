/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {fireEvent, render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import {ENumberFormatPolicy, NumberFormatProvider} from "@sequentech/ui-core"
import theme from "../../services/theme"
import {PreferentialCandidateResults} from "./PreferentialCandidateResults"
import type {PreferentialProcessResults, PreferentialRound} from "./types"

// ui-core's built dist is unavailable when this package's tests run alone, so
// load the modules the results use from source.
jest.mock(
    "@sequentech/ui-core",
    () => ({
        ...jest.requireActual<typeof import("../../../../ui-core/src/types/VotingChannel")>(
            "../../../../ui-core/src/types/VotingChannel"
        ),
        ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
        ...jest.requireActual("../../../../ui-core/src/services/numberFormat"),
        ...jest.requireActual("../../../../ui-core/src/services/NumberFormatContext"),
        ...jest.requireActual("../../../../ui-core/src/services/percentFormatter"),
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

const candidateA = {id: "candidate-a", name: "Candidate A"}
const candidateB = {id: "candidate-b", name: "Candidate B"}

const processResults: PreferentialProcessResults = {
    name_references: [candidateA, candidateB],
    round_count: 1,
    max_rounds: 1,
    rounds: [
        {
            winner: candidateA,
            candidates_wins: {
                [candidateA.id]: {
                    name: candidateA.name,
                    wins: 1234567,
                    transference: 0,
                    percentage: 0.456789,
                },
                [candidateB.id]: {
                    name: candidateB.name,
                    wins: 999,
                    transference: 0,
                    percentage: 0.000809,
                },
            },
            eliminated_candidates: null,
            active_candidates_count: 2,
            active_ballots_count: 1235566,
            exhausted_ballots_count: 0,
        },
    ],
}

// The visible text of each round's votes cell, without the separators React
// writes between adjacent text nodes.
const roundVotes = (markup: string): string[] =>
    Array.from(
        markup
            .replace(/<!-- -->/g, "")
            .matchAll(/seq-tally-results-preferential-results__round-votes[^>]*>([^<]*)</g),
        (match) => match[1]
    )

describe("PreferentialCandidateResults", () => {
    it("writes round votes and percentages in the event's number format", () => {
        const markup = renderToStaticMarkup(
            <NumberFormatProvider policy={ENumberFormatPolicy.PERIOD_COMMA}>
                <PreferentialCandidateResults
                    processResults={processResults}
                    candidates={[candidateA, candidateB]}
                />
            </NumberFormatProvider>
        )

        expect(roundVotes(markup)).toEqual(["1.234.567 (45,68%)", "999 (0,08%)"])
    })

    it("keeps the comma grouped output for events without a number format", () => {
        const markup = renderToStaticMarkup(
            <PreferentialCandidateResults
                processResults={processResults}
                candidates={[candidateA, candidateB]}
            />
        )

        expect(roundVotes(markup)).toEqual(["1,234,567 (45.68%)", "999 (0.08%)"])
    })
})
