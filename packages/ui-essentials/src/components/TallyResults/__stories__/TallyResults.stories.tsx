// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {expect, userEvent, within} from "storybook/test"
import {Meta, StoryObj} from "@storybook/react-vite"
import {INITIAL_VIEWPORTS} from "storybook/viewport"
import ResultsAndParticipation, {
    CandidateResultRow,
    ResultsParticipationSummary,
} from "../TallyResults"

const summary: ResultsParticipationSummary = {
    id: "summary",
    eligibleCensus: 12500,
    totalAuditableVotes: 8750,
    totalAuditableVotesPercent: 0.7,
    totalVotes: 8725,
    totalVotesPercent: 0.698,
    totalValidVotes: 8420,
    totalValidVotesPercent: 0.6736,
    totalInvalidVotes: 305,
    totalInvalidVotesPercent: 0.024399999999999998,
    explicitInvalidVotes: 180,
    explicitInvalidVotesPercent: 0.0144,
    implicitInvalidVotes: 125,
    implicitInvalidVotesPercent: 0.01,
    blankVotes: 540,
    blankVotesPercent: 0.0432,
}

const candidates: CandidateResultRow[] = [
    {
        id: "candidate-1",
        name: "Avery Chen",
        castVotes: 3280,
        castVotesPercent: 0.3895,
        winningPosition: 1,
    },
    {
        id: "candidate-2",
        name: "Marta Alvarez",
        castVotes: 2910,
        castVotesPercent: 0.3456,
        winningPosition: 2,
    },
    {
        id: "candidate-3",
        name: "Nadia Johnson",
        castVotes: 1690,
        castVotesPercent: 0.2007,
        winningPosition: null,
    },
]

const meta: Meta<typeof ResultsAndParticipation> = {
    title: "components/TallyResults/ResultsAndParticipation",
    component: ResultsAndParticipation,
    parameters: {
        expectedFailure: {
            reason: "Participation summary renders an empty column header.",
            a11y: ["empty-table-header"],
        },
        backgrounds: {
            default: "white",
        },
        viewport: {
            viewports: INITIAL_VIEWPORTS,
            defaultViewport: "responsive",
        },
    },
    args: {
        chartName: "City Council - District 1",
        summary,
        candidates,
    },
}

export default meta

type Story = StoryObj<typeof ResultsAndParticipation>

export const Base: Story = {}

export const Mobile: Story = {
    parameters: {
        viewport: {
            defaultViewport: "iphone6",
        },
    },
}

export const MissingParticipation: Story = {
    args: {summary: null},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("No items")).toBeVisible()
        const row = canvas.getByRole("row", {name: /Avery Chen/})
        await expect(row).toHaveTextContent("3280")
        await expect(row).toHaveTextContent("38.95%")
    },
}
export const MissingValues: Story = {
    args: {summary: {eligibleCensus: 100}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const row = canvas.getByRole("row", {name: /Total Votes Counted/})
        await expect(
            within(row)
                .getAllByRole("cell")
                .map((cell) => cell.textContent)
        ).toEqual(["-", "-"])
    },
}
export const PreferentialRounds: Story = {
    args: {
        summary: null,
        preferential: true,
        processResults: {
            name_references: candidates.map(({id, name}) => ({id, name})),
            round_count: 5,
            max_rounds: 5,
            rounds: Array.from({length: 5}, (_, index) => ({
                winner: null,
                eliminated_candidates: [],
                active_candidates_count: 3,
                active_ballots_count: 100,
                exhausted_ballots_count: index,
                candidates_wins: Object.fromEntries(
                    candidates.map(({id, name}) => [
                        id,
                        {name, wins: 30 + index, transference: index, percentage: 30 + index},
                    ])
                ),
            })),
        },
    },
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("columnheader", {name: /Round 1/})).toBeVisible()
        for (let step = 0; step < 3; step++)
            await userEvent.click(canvas.getByRole("button", {name: "Navigate to next rounds"}))
        await expect(canvas.getByRole("columnheader", {name: /Round 5/})).toBeVisible()
        for (let step = 0; step < 3; step++)
            await userEvent.click(canvas.getByRole("button", {name: "Navigate to previous rounds"}))
        await expect(canvas.getByRole("columnheader", {name: /Round 1/})).toBeVisible()
    },
}
