// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {resultsFixture} from "../../../tests/fixtures/results"
import {ContestResultsBlock} from "../ContestResultsBlock"

const fixture = resultsFixture()
const meta = {
    title: "Results/Contest",
    component: ContestResultsBlock,
    args: {manifestContest: fixture.manifest.contests[0], dataset: fixture.dataset, locale: "en"},
    decorators: [
        (Story) => (
            <main>
                <h1>Election results</h1>
                <h2>Contest results</h2>
                <Story />
            </main>
        ),
    ],
    parameters: {
        expectedFailure: {
            reason: "The shared participation summary renders an empty column header.",
            a11y: ["empty-table-header"],
        },
    },
} satisfies Meta<typeof ContestResultsBlock>
export default meta
type Story = StoryObj<typeof meta>

export const PublishedWinner: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Council representative"})).toBeVisible()
        const alice = within(canvas.getByRole("row", {name: /Alice Example/}))
        await expect(alice.getByRole("gridcell", {name: "45"})).toBeVisible()
        await expect(alice.getByRole("gridcell", {name: "60.00%"})).toBeVisible()
        await expect(alice.getByRole("gridcell", {name: "1"})).toBeVisible()
        await expect(canvas.getByRole("row", {name: /Bob Example/})).toHaveTextContent("30")
    },
}

export const Unpublished: Story = {
    args: {manifestContest: {...fixture.manifest.contests[0], publication_state: "not_published"}},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getAllByText("Not published yet")[0]).toBeVisible()
        await expect(canvas.queryByRole("grid")).not.toBeInTheDocument()
        await expect(canvas.queryByText("Alice Example")).not.toBeInTheDocument()
    },
}

export const MissingParticipation: Story = {
    args: {dataset: {...fixture.dataset, results_contest: []}},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("No items")).toBeVisible()
        await expect(canvas.getByRole("row", {name: /Alice Example/})).toHaveTextContent("45")
    },
}

export const MissingValues: Story = {
    args: {
        dataset: {
            ...fixture.dataset,
            results_contest: [
                {
                    id: "missing",
                    election_id: "council",
                    contest_id: "representative",
                    elegible_census: 100,
                },
            ],
        },
    },
    play: async ({canvasElement}) => {
        const row = within(canvasElement).getByRole("row", {name: /Total Votes Counted/})
        await expect(
            within(row)
                .getAllByRole("cell")
                .map((cell) => cell.textContent)
        ).toEqual(["-", "-"])
    },
}

const tied = resultsFixture().dataset
tied.results_contest[0] = {...tied.results_contest[0], total_votes: 65, total_valid_votes: 60}
tied.results_contest_candidate = tied.results_contest_candidate.map((candidate) => ({
    ...candidate,
    cast_votes: 30,
    cast_votes_percent: 0.5,
    winning_position: 1,
}))
export const TiedWinners: Story = {
    args: {dataset: tied},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        for (const name of [/Alice Example/, /Bob Example/]) {
            const row = within(canvas.getByRole("row", {name}))
            await expect(row.getByRole("gridcell", {name: "30"})).toBeVisible()
            await expect(row.getByRole("gridcell", {name: "50.00%"})).toBeVisible()
            await expect(row.getByRole("gridcell", {name: "1"})).toBeVisible()
        }
    },
}

export const Acclaimed: Story = {
    args: {
        dataset: {
            ...fixture.dataset,
            contest: fixture.dataset.contest.map((contest) => ({...contest, is_acclaimed: true})),
            results_contest_candidate: fixture.dataset.results_contest_candidate.map(
                (candidate) => ({
                    ...candidate,
                    cast_votes: 0,
                    cast_votes_percent: 0,
                    winning_position: 1,
                })
            ),
        },
    },
    parameters: {
        expectedFailure: {
            reason: "The acclaimed chip uses blue text with contrast 3.85 on white, below the required 4.5.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(
                "Won by acclamation. This contest was decided without a vote, so no votes were recorded for it."
            )
        ).toBeVisible()
        await expect(canvas.queryByText("Participation Summary")).not.toBeInTheDocument()
        await expect(canvas.getByRole("row", {name: /Alice Example/})).toHaveTextContent("0")
    },
}

const preferential = resultsFixture().dataset
preferential.contest[0].counting_algorithm = "instant-runoff"
preferential.results_contest[0].annotations = {
    process_results: {
        name_references: [
            {id: "alice", name: "Alice Example"},
            {id: "bob", name: "Bob Example"},
        ],
        round_count: 5,
        max_rounds: 5,
        rounds: Array.from({length: 5}, (_, index) => ({
            winner: index === 4 ? "alice" : null,
            eliminated_candidates: [],
            active_candidates_count: 2,
            active_ballots_count: 75,
            exhausted_ballots_count: index,
            candidates_wins: {
                alice: {
                    name: "Alice Example",
                    wins: 40 + index,
                    transference: index,
                    percentage: 50 + index,
                },
                bob: {
                    name: "Bob Example",
                    wins: 35 - index,
                    transference: -index,
                    percentage: 50 - index,
                },
            },
        })),
    },
}
export const PreferentialRoundNavigation: Story = {
    args: {dataset: preferential},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("columnheader", {name: /Round 1/})).toBeVisible()
        for (let round = 0; round < 3; round++)
            await userEvent.click(canvas.getByRole("button", {name: "Navigate to next rounds"}))
        await expect(canvas.getByRole("columnheader", {name: /Round 5/})).toBeVisible()
        await expect(canvas.getByRole("row", {name: /Alice Example/})).toHaveTextContent("44")
        for (let round = 0; round < 3; round++)
            await userEvent.click(canvas.getByRole("button", {name: "Navigate to previous rounds"}))
        await expect(canvas.getByRole("columnheader", {name: /Round 1/})).toBeVisible()
    },
}
