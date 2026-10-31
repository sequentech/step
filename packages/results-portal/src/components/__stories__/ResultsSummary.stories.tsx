// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {resultsFixture} from "../../../tests/fixtures/results"
import {ResultsSummary} from "../ResultsSummary"

const fixture = resultsFixture()
const meta = {
    title: "Results/General information",
    component: ResultsSummary,
    args: {
        elections: fixture.dataset.election,
        resultsElections: fixture.dataset.results_election,
        locale: "en",
    },
    decorators: [
        (Story) => (
            <main>
                <h1>Election results</h1>
                <Story />
            </main>
        ),
    ],
} satisfies Meta<typeof ResultsSummary>
export default meta
type Story = StoryObj<typeof meta>

export const ParticipationAndBlankBallots: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const table = within(canvas.getByRole("table", {name: "General results information"}))
        await expect(table.getByRole("columnheader", {name: "Total blank ballots"})).toBeVisible()
        const row = within(table.getByRole("row", {name: /Community Council/}))
        await expect(row.getAllByRole("cell").map((cell) => cell.textContent)).toEqual([
            "Community Council",
            "100",
            "80",
            "3",
            "80.00%",
        ])
    },
}

export const MissingCountsAreDashes: Story = {
    args: {resultsElections: [{id: "unknown", election_id: "council"}]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.queryByRole("columnheader", {name: "Total blank ballots"})
        ).not.toBeInTheDocument()
        const row = within(canvas.getByRole("row", {name: /Community Council/}))
        await expect(row.getAllByRole("cell").map((cell) => cell.textContent)).toEqual([
            "Community Council",
            "-",
            "-",
            "-",
        ])
    },
}
