// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {Box} from "@mui/material"
import {
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Area_Contest,
} from "../../gql/graphql"
import {Sequent_Backend_Candidate_Extended} from "./types"
import Chart, {Props} from "react-apexcharts"
import CardChart from "@/components/dashboard/charts/Charts"

const MAX_CANDIDATES_REPRESENTED = 5

interface ParticipationSummaryChartProps {
    result: Sequent_Backend_Results_Contest | Sequent_Backend_Results_Area_Contest
    chartName: string
}

export const ParticipationSummaryChart: React.FC<ParticipationSummaryChartProps> = ({
    result,
    chartName,
}) => {
    const {t} = useTranslation()
    if (result.elegible_census === 0) {
        return null
    }

    const eligibleCensus = result.elegible_census as number
    const validVotes = result.total_valid_votes as number
    const invalidVotes = result.total_invalid_votes as number
    const blankVotes = result.blank_votes as number
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

    const chartOptions: Props = {
        options: {
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
        },
        series: chartData.map((item) => item.value),
    }

    return (
        <Box
            key={result.id}
            sx={{mb: 2, border: "1px solid #cccccc99", maxWidth: {xs: "100%", md: 450}}}
        >
            <CardChart title={chartName} collapsible={true}>
                <Chart
                    options={chartOptions.options}
                    series={chartOptions.series}
                    type="pie"
                    width="100%"
                    height={300}
                />
            </CardChart>
        </Box>
    )
}

interface CandidatesResultsChartsProps {
    results: Sequent_Backend_Candidate_Extended[]
    chartName: string
}

export const CandidatesResultsCharts: React.FC<CandidatesResultsChartsProps> = ({
    results,
    chartName,
}) => {
    if (!results || results.length === 0) {
        return null
    }

    let chartData = [
        ...results.map((candidate, index) => {
            let castVotes = (candidate.cast_votes ?? 0) as number
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
        let deletedItems = chartData.splice(MAX_CANDIDATES_REPRESENTED)
        let othersSum = deletedItems.reduce((a, b) => a + b.value, 0)
        chartData.push({label: "Others", value: othersSum})
    }

    const chartOptions: Props = {
        options: {
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
        },
        series: chartData.map((item) => item.value),
    }

    return (
        <Box
            key={chartName + "-candidates"}
            sx={{mb: 2, border: "1px solid #cccccc99", maxWidth: {xs: "100%", md: 450}}}
        >
            <CardChart title={chartName} collapsible={true}>
                <Chart
                    options={chartOptions.options}
                    series={chartOptions.series}
                    type="pie"
                    width="100%"
                    height={300}
                />
            </CardChart>
        </Box>
    )
}
