// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo} from "react"
import {
    Box,
    Paper,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import type {Props as ApexChartProps} from "react-apexcharts"
import {RESPONSIVE_PIE_OPTIONS} from "./constants"
import {Chart, ChartPanel} from "./ChartPanel"
import type {
    NumericValue,
    ResultsAndParticipationLabels,
    ResultsParticipationSummary,
} from "./types"
import {mergeLabels, percentOrDash, toFiniteNumber, valueOrDash} from "./utils"

interface ParticipationSummaryChartProps {
    result: ResultsParticipationSummary
    chartName: string
    labels?: Partial<ResultsAndParticipationLabels>
}

interface ParticipationSummaryProps {
    result?: ResultsParticipationSummary | null
    chartName: string
    labels?: Partial<ResultsAndParticipationLabels>
    showWeight?: boolean
}

interface SummaryRow {
    label: string
    value?: NumericValue
    percent?: NumericValue
    showPercent?: boolean
}

const buildParticipationChartData = (
    result: ResultsParticipationSummary,
    labels: ResultsAndParticipationLabels
) => {
    const eligibleCensus = toFiniteNumber(result.eligibleCensus)
    if (!eligibleCensus) return []

    const validVotes = toFiniteNumber(result.totalValidVotes)
    const invalidVotes = toFiniteNumber(result.totalInvalidVotes)
    const blankVotes = toFiniteNumber(result.blankVotes)
    const totalVotes = toFiniteNumber(result.totalVotes)
    const totalAuditableVotes = toFiniteNumber(result.totalAuditableVotes)
    const countedVotes =
        totalVotes ??
        (validVotes !== null || invalidVotes !== null
            ? (validVotes ?? 0) + (invalidVotes ?? 0)
            : totalAuditableVotes)

    if (countedVotes === null) return []

    const representedInvalidVotes = invalidVotes ?? 0
    const representedBlankVotes = blankVotes ?? 0
    const votesForCandidatesBase =
        totalVotes !== null ? countedVotes - representedInvalidVotes : (validVotes ?? countedVotes)
    const votesForCandidates = Math.max(votesForCandidatesBase - representedBlankVotes, 0)
    const nonVoters = Math.max(eligibleCensus - countedVotes, 0)

    return [
        {
            label: labels.votesForCandidates,
            value: votesForCandidates,
        },
        {
            label: labels.blankVotesChart,
            value: representedBlankVotes,
        },
        {
            label: labels.invalidVotes,
            value: representedInvalidVotes,
        },
        {
            label: labels.nonVoters,
            value: nonVoters,
        },
    ].filter((item) => item.value > 0)
}

const buildSummaryRows = (
    result: ResultsParticipationSummary,
    labels: ResultsAndParticipationLabels,
    showWeight: boolean
): SummaryRow[] => {
    const rows: SummaryRow[] = [
        {
            label: labels.eligibleCensus,
            value: result.eligibleCensus,
            showPercent: false,
        },
        {
            label: labels.totalAuditableVotes,
            value: result.totalAuditableVotes,
            percent: result.totalAuditableVotesPercent,
        },
        {
            label: labels.totalVotesCounted,
            value: result.totalVotes,
            percent: result.totalVotesPercent,
        },
        {
            label: labels.totalValidVotes,
            value: result.totalValidVotes,
            percent: result.totalValidVotesPercent,
        },
        {
            label: labels.totalInvalidVotes,
            value: result.totalInvalidVotes,
            percent: result.totalInvalidVotesPercent,
        },
        {
            label: labels.explicitInvalidVotes,
            value: result.explicitInvalidVotes,
            percent: result.explicitInvalidVotesPercent,
        },
        {
            label: labels.implicitInvalidVotes,
            value: result.implicitInvalidVotes,
            percent: result.implicitInvalidVotesPercent,
        },
        {
            label: labels.blankVotes,
            value: result.blankVotes,
            percent: result.blankVotesPercent,
        },
    ]

    if (showWeight) {
        rows.push({
            label: labels.weight,
            value: result.weight,
            showPercent: false,
        })
    }

    return rows
}

export const ParticipationSummaryChart: React.FC<ParticipationSummaryChartProps> = ({
    result,
    chartName,
    labels,
}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const chartData = useMemo(
        () => buildParticipationChartData(result, mergedLabels),
        [result, mergedLabels]
    )
    const chartOptions = useMemo<ApexChartProps>(
        () => ({
            options: {
                labels: chartData.map((item) => item.label),
                legend: {
                    position: "right",
                },
                responsive: RESPONSIVE_PIE_OPTIONS,
            },
            series: chartData.map((item) => item.value),
        }),
        [chartData]
    )

    if (chartData.length === 0) {
        return null
    }

    return (
        <ChartPanel title={chartName} className="seq-tally-results-participation-chart">
            <Chart
                className="seq-tally-results-participation-chart__chart"
                options={chartOptions.options}
                series={chartOptions.series}
                type="pie"
                width="100%"
                height={300}
            />
        </ChartPanel>
    )
}

export const ParticipationSummary: React.FC<ParticipationSummaryProps> = ({
    result,
    chartName,
    labels,
    showWeight = false,
}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const summaryRows = useMemo(
        () => (result ? buildSummaryRows(result, mergedLabels, showWeight) : []),
        [result, mergedLabels, showWeight]
    )

    return (
        <Box
            className="seq-tally-results-participation-summary"
            sx={{borderTop: "1px solid", borderColor: "divider", mt: 4, p: 0}}
        >
            <Typography
                className="seq-tally-results-participation-summary__title"
                variant="h6"
                component="div"
                sx={{mt: 6, ml: 1}}
            >
                {mergedLabels.participationSummary}
            </Typography>

            {result ? (
                <Box
                    className="seq-tally-results-participation-summary__content"
                    sx={{
                        display: "flex",
                        flexDirection: {xs: "column", lg: "row"},
                        gap: 4,
                        alignItems: "flex-start",
                    }}
                >
                    <Box
                        className="seq-tally-results-participation-summary__chart-column"
                        sx={{
                            flex: {xs: "1 1 auto", lg: "0 0 450px"},
                            mt: 2,
                            width: {xs: "100%", lg: 450},
                            maxWidth: "100%",
                        }}
                    >
                        <ParticipationSummaryChart
                            result={result}
                            chartName={chartName}
                            labels={mergedLabels}
                        />
                    </Box>
                    <Box
                        className="seq-tally-results-participation-summary__table-column"
                        sx={{
                            flex: "1 1 auto",
                            mt: 2,
                            border: "1px solid",
                            borderColor: "divider",
                            minWidth: 0,
                            width: "100%",
                        }}
                    >
                        <TableContainer
                            className="seq-tally-results-participation-summary__table-container"
                            component={Paper}
                        >
                            <Table
                                className="seq-tally-results-participation-summary__table"
                                sx={{minWidth: {xs: 300, sm: 500}, tableLayout: "fixed"}}
                            >
                                <TableHead>
                                    <TableRow>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__label-heading"
                                            sx={{
                                                width: {xs: "48%", sm: "44%"},
                                                overflowWrap: "anywhere",
                                            }}
                                        />
                                        <TableCell
                                            className="seq-tally-results-participation-summary__total-heading"
                                            sx={{width: {xs: "26%", sm: "28%"}}}
                                            align="right"
                                        >
                                            {mergedLabels.total}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__turnout-heading"
                                            sx={{width: {xs: "26%", sm: "28%"}}}
                                            align="right"
                                        >
                                            {mergedLabels.turnout}
                                        </TableCell>
                                    </TableRow>
                                </TableHead>
                                <TableBody>
                                    {summaryRows.map((row) => (
                                        <TableRow key={row.label}>
                                            <TableCell
                                                className="seq-tally-results-participation-summary__label-cell"
                                                component="th"
                                                scope="row"
                                                sx={{overflowWrap: "anywhere"}}
                                            >
                                                {row.label}
                                            </TableCell>
                                            <TableCell
                                                className="seq-tally-results-participation-summary__value-cell"
                                                align="right"
                                            >
                                                {valueOrDash(row.value)}
                                            </TableCell>
                                            <TableCell
                                                className="seq-tally-results-participation-summary__percent-cell"
                                                align="right"
                                            >
                                                {row.showPercent === false
                                                    ? ""
                                                    : percentOrDash(row.percent)}
                                            </TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                            </Table>
                        </TableContainer>
                    </Box>
                </Box>
            ) : (
                <Typography
                    className="seq-tally-results-participation-summary__empty"
                    color="text.secondary"
                    sx={{mt: 2}}
                >
                    {mergedLabels.empty}
                </Typography>
            )}
        </Box>
    )
}
