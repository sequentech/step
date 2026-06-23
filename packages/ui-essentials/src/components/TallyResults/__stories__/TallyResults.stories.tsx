// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import ResultsAndParticipation, {
    CandidateResultRow,
    ResultsParticipationSummary,
} from "../TallyResults"

const summary: ResultsParticipationSummary = {
    id: "summary",
    eligibleCensus: 12500,
    totalAuditableVotes: 8750,
    totalAuditableVotesPercent: 70,
    totalVotes: 8725,
    totalVotesPercent: 69.8,
    totalValidVotes: 8420,
    totalValidVotesPercent: 67.36,
    totalInvalidVotes: 305,
    totalInvalidVotesPercent: 2.44,
    explicitInvalidVotes: 180,
    explicitInvalidVotesPercent: 1.44,
    implicitInvalidVotes: 125,
    implicitInvalidVotesPercent: 1,
    blankVotes: 540,
    blankVotesPercent: 4.32,
}

const candidates: CandidateResultRow[] = [
    {
        id: "candidate-1",
        name: "Avery Chen",
        castVotes: 3280,
        castVotesPercent: 38.95,
        winningPosition: 1,
    },
    {
        id: "candidate-2",
        name: "Marta Alvarez",
        castVotes: 2910,
        castVotesPercent: 34.56,
        winningPosition: 2,
    },
    {
        id: "candidate-3",
        name: "Nadia Johnson",
        castVotes: 1690,
        castVotesPercent: 20.07,
        winningPosition: null,
    },
]

const meta: Meta<typeof ResultsAndParticipation> = {
    title: "components/TallyResults/ResultsAndParticipation",
    component: ResultsAndParticipation,
    parameters: {
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
