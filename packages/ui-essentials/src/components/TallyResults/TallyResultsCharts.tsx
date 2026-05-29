// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//
// Lifted from admin-portal/src/resources/Tally/TallyResultsCharts.tsx.
// Adaptations:
//   C1: replace `useTranslation()` with the `t()` shim from ./strings (see L1).
//   C2: drop `Sequent_Backend_*` types from ParticipationSummaryChartProps;
//       accept a plain TallyParticipationSummary instead.
//   C3: drop `Sequent_Backend_Candidate_Extended[]` from CandidatesResultsChartsProps;
//       accept TallyCandidate[] instead. Renamed `results` prop to `candidates`
//       (TallyCandidate[] is a clearer term than `results`).
//   C4: replace `Props` from react-apexcharts (removed in react-apexcharts >=1.7)
//       with `ApexOptions` from `apexcharts`.
//   C5: replace admin-portal's CardChart wrapper with a plain MUI Box +
//       Typography heading (admin-portal's CardChart is a project-local
//       component that pulls in collapsibility we don't need here).

import React from "react"
import {Box, Typography} from "@mui/material"
import Chart from "react-apexcharts"
import type {ApexOptions} from "apexcharts"

import {TallyCandidate, TallyParticipationSummary} from "./types"
import {t} from "./strings"

const MAX_CANDIDATES_REPRESENTED = 5

interface ParticipationSummaryChartProps {
    result: TallyParticipationSummary
    chartName: string
}

export const ParticipationSummaryChart: React.FC<ParticipationSummaryChartProps> = ({
    result,
    chartName,
}) => {
    if (result.elegible_census === 0) {
        return null
    }

    const eligibleCensus = result.elegible_census
    const validVotes = result.total_valid_votes
    const invalidVotes = result.total_invalid_votes
    const blankVotes = result.blank_votes
    const VotesForCandidates = validVotes - blankVotes
    const nonVoters = eligibleCensus - validVotes - invalidVotes
    const chartData = [
        {
            label: t("tally.chart.votesForCandidates"),
            value: VotesForCandidates,
        },
        {
            label: t("tally.chart.blankVotes"),
            value: blankVotes,
        },
        {
            label: t("tally.chart.invalidVotes"),
            value: invalidVotes,
        },
        {
            label: t("tally.chart.nonVoters"),
            value: nonVoters,
        },
    ].filter((item) => item.value > 0)

    const chartOptions: ApexOptions = {
        labels: chartData.map((item) => item.label),
        legend: {
            position: "right",
        },
        responsive: [
            {
                breakpoint: 480,
                options: {
                    chart: {
                        width: 200,
                    },
                    legend: {
                        position: "bottom",
                    },
                },
            },
        ],
    }
    const series = chartData.map((item) => item.value)

    return (
        <Box
            sx={{mb: 2, border: "1px solid #cccccc99", maxWidth: {xs: "100%", md: 450}, p: 2}}
        >
            <Typography variant="subtitle1" sx={{mb: 1}}>
                {chartName}
            </Typography>
            <Chart
                key={series.join(",")}
                options={chartOptions}
                series={series}
                type="pie"
                width="100%"
                height={300}
            />
        </Box>
    )
}

interface CandidatesResultsChartsProps {
    candidates: TallyCandidate[]
    chartName: string
}

export const CandidatesResultsCharts: React.FC<CandidatesResultsChartsProps> = ({
    candidates,
    chartName,
}) => {
    if (!candidates || candidates.length === 0) {
        return null
    }

    let chartData = [
        ...candidates.map((candidate) => {
            const castVotes = (candidate.cast_votes ?? 0) as number
            return {
                label: candidate.name ?? "-",
                value: castVotes,
            }
        }),
    ]
    chartData = chartData
        .filter((item) => item.value && item.value > 0)
        .sort((a, b) => b.value - a.value)

    let totalCandidatesRepresented = chartData ? chartData?.length : 0
    if (totalCandidatesRepresented === 0) {
        return null
    } else if (totalCandidatesRepresented > MAX_CANDIDATES_REPRESENTED) {
        totalCandidatesRepresented = MAX_CANDIDATES_REPRESENTED
        // Trim chartData to represent only the first 5 candidates + "Others"
        const deletedItems = chartData.splice(MAX_CANDIDATES_REPRESENTED)
        const othersSum = deletedItems.reduce((a, b) => a + b.value, 0)
        chartData.push({label: "Others", value: othersSum})
    }

    const chartOptions: ApexOptions = {
        labels: chartData.map((item) => item.label),
        legend: {
            position: "right",
        },
        responsive: [
            {
                breakpoint: 480,
                options: {
                    chart: {
                        width: 200,
                    },
                    legend: {
                        position: "bottom",
                    },
                },
            },
        ],
        // Six colors, starting with the same blue than the other charts above
        colors: ["#008FFBFF", "#FF0000", "#dfdf01ff", "#079107ff", "#FF8000", "#706565ff"],
    }
    const series = chartData.map((item) => item.value)

    return (
        <Box
            sx={{mb: 2, border: "1px solid #cccccc99", maxWidth: {xs: "100%", md: 450}, p: 2}}
        >
            <Typography variant="subtitle1" sx={{mb: 1}}>
                {chartName}
            </Typography>
            <Chart
                key={series.join(",")}
                options={chartOptions}
                series={series}
                type="pie"
                width="100%"
                height={300}
            />
        </Box>
    )
}
