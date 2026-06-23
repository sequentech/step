// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useState} from "react"
import {
    Box,
    Chip,
    Paper,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Typography,
    useMediaQuery,
    useTheme,
} from "@mui/material"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import ReactApexChart, {Props as ApexChartProps} from "react-apexcharts"
import {formatPercentOne} from "@sequentech/ui-core"

type NumericValue = number | string | null | undefined

const MAX_CANDIDATES_REPRESENTED = 5
const Chart = ((ReactApexChart as unknown as {default?: React.ComponentType<ApexChartProps>})
    .default ?? ReactApexChart) as React.ComponentType<ApexChartProps>
const DATA_GRID_INITIAL_STATE = {
    pagination: {
        paginationModel: {
            pageSize: 10,
        },
    },
}
const DATA_GRID_PAGE_SIZE_OPTIONS = [10, 20, 50, 100]
const RESPONSIVE_PIE_OPTIONS = [
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
]

export interface ResultsParticipationSummary {
    id?: string | number
    eligibleCensus?: NumericValue
    totalAuditableVotes?: NumericValue
    totalAuditableVotesPercent?: NumericValue
    totalVotes?: NumericValue
    totalVotesPercent?: NumericValue
    totalValidVotes?: NumericValue
    totalValidVotesPercent?: NumericValue
    totalInvalidVotes?: NumericValue
    totalInvalidVotesPercent?: NumericValue
    explicitInvalidVotes?: NumericValue
    explicitInvalidVotesPercent?: NumericValue
    implicitInvalidVotes?: NumericValue
    implicitInvalidVotesPercent?: NumericValue
    blankVotes?: NumericValue
    blankVotesPercent?: NumericValue
    weight?: NumericValue
}

export interface CandidateResultRow {
    id: string
    name: string
    castVotes?: NumericValue
    castVotesPercent?: NumericValue
    winningPosition?: NumericValue
}

export interface CandidateReference {
    id: string
    name: string
}

export interface CandidateOutcome {
    name: string
    wins: number
    transference: number
    percentage: number
}

export type CandidatesOutcomes = Record<string, CandidateOutcome>

export interface PreferentialRound {
    winner: CandidateReference | null
    candidates_wins: CandidatesOutcomes
    eliminated_candidates: CandidateReference[] | null
    active_candidates_count: number
    active_ballots_count: number
    exhausted_ballots_count: number
}

export interface PreferentialProcessResults {
    candidates_status?: Record<string, string>
    name_references: CandidateReference[]
    round_count: number
    rounds: PreferentialRound[]
    max_rounds: number
}

export interface ResultsAndParticipationLabels {
    participationSummary: string
    candidateResults: string
    total: string
    turnout: string
    eligibleCensus: string
    totalAuditableVotes: string
    totalVotesCounted: string
    totalValidVotes: string
    totalInvalidVotes: string
    explicitInvalidVotes: string
    implicitInvalidVotes: string
    blankVotes: string
    blankVotesChart: string
    weight: string
    options: string
    castVotes: string
    castVotesPercent: string
    winningPosition: string
    votesForCandidates: string
    invalidVotes: string
    nonVoters: string
    others: string
    candidate: string
    round: string
    winner: string
    eliminated: string
    empty: string
    previousRounds: string
    nextRounds: string
}

export interface ResultsAndParticipationProps {
    chartName: string
    summary?: ResultsParticipationSummary | null
    candidates: CandidateResultRow[]
    labels?: Partial<ResultsAndParticipationLabels>
    showWeight?: boolean
    processResults?: PreferentialProcessResults | null
}

export const defaultResultsAndParticipationLabels: ResultsAndParticipationLabels = {
    participationSummary: "Participation Summary",
    candidateResults: "Candidate Results",
    total: "Total",
    turnout: "%",
    eligibleCensus: "Eligible Voters",
    totalAuditableVotes: "Total Auditable Votes",
    totalVotesCounted: "Total Votes Counted",
    totalValidVotes: "Total Valid Votes",
    totalInvalidVotes: "Total Invalid Votes",
    explicitInvalidVotes: "Explicitly Invalid Votes",
    implicitInvalidVotes: "Implicitly Invalid Votes",
    blankVotes: "Blank Votes",
    blankVotesChart: "Blank Votes",
    weight: "Weight",
    options: "Options",
    castVotes: "Number of Votes",
    castVotesPercent: "Percent of Votes",
    winningPosition: "Winning position",
    votesForCandidates: "Votes For Candidates",
    invalidVotes: "Invalid Votes",
    nonVoters: "Non Voters",
    others: "Others",
    candidate: "Candidate",
    round: "Round",
    winner: "Winner",
    eliminated: "Eliminated",
    empty: "No items",
    previousRounds: "Navigate to previous rounds",
    nextRounds: "Navigate to next rounds",
}

const mergeLabels = (
    labels?: Partial<ResultsAndParticipationLabels>
): ResultsAndParticipationLabels => ({
    ...defaultResultsAndParticipationLabels,
    ...labels,
})

const toFiniteNumber = (value: NumericValue): number | null => {
    if (typeof value === "number") {
        return Number.isFinite(value) ? value : null
    }

    if (typeof value === "string") {
        const trimmed = value.trim()
        if (!trimmed) return null

        const parsed = Number(trimmed)
        return Number.isFinite(parsed) ? parsed : null
    }

    return null
}

const valueOrDash = (value: NumericValue): string | number => toFiniteNumber(value) ?? "-"

const percentOrDash = (value: NumericValue): string => {
    const numeric = toFiniteNumber(value)
    return numeric !== null ? formatPercentOne(numeric) : "-"
}

export const sortCandidateResults = (
    left: CandidateResultRow,
    right: CandidateResultRow
): number => {
    const leftWinning = toFiniteNumber(left.winningPosition) ?? Number.MAX_SAFE_INTEGER
    const rightWinning = toFiniteNumber(right.winningPosition) ?? Number.MAX_SAFE_INTEGER

    if (leftWinning !== rightWinning) {
        return leftWinning - rightWinning
    }

    return (toFiniteNumber(right.castVotes) ?? 0) - (toFiniteNumber(left.castVotes) ?? 0)
}

const ChartPanel: React.FC<{title: string; children: React.ReactNode; className?: string}> = ({
    title,
    children,
    className,
}) => (
    <Paper
        className={["seq-tally-results-chart-panel", className].filter(Boolean).join(" ")}
        variant="outlined"
        sx={{
            p: 2,
            width: "100%",
            maxWidth: {xs: "100%", md: 450},
        }}
    >
        <Typography
            className="seq-tally-results-chart-panel__title"
            variant="subtitle1"
            sx={{fontWeight: 600}}
        >
            {title}
        </Typography>
        <Box
            className="seq-tally-results-chart-panel__body"
            sx={{borderTop: "1px solid", borderColor: "divider", mt: 2, pt: 2}}
        >
            {children}
        </Box>
    </Paper>
)

export const ParticipationSummaryChart: React.FC<{
    result: ResultsParticipationSummary
    chartName: string
    labels?: Partial<ResultsAndParticipationLabels>
}> = ({result, chartName, labels}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const chartData = useMemo(() => {
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
            totalVotes !== null
                ? countedVotes - representedInvalidVotes
                : (validVotes ?? countedVotes)
        const votesForCandidates = Math.max(votesForCandidatesBase - representedBlankVotes, 0)
        const nonVoters = Math.max(eligibleCensus - countedVotes, 0)

        return [
            {
                label: mergedLabels.votesForCandidates,
                value: votesForCandidates,
            },
            {
                label: mergedLabels.blankVotesChart,
                value: representedBlankVotes,
            },
            {
                label: mergedLabels.invalidVotes,
                value: representedInvalidVotes,
            },
            {
                label: mergedLabels.nonVoters,
                value: nonVoters,
            },
        ].filter((item) => item.value > 0)
    }, [
        result.eligibleCensus,
        result.totalValidVotes,
        result.totalInvalidVotes,
        result.blankVotes,
        result.totalVotes,
        result.totalAuditableVotes,
        mergedLabels.votesForCandidates,
        mergedLabels.blankVotesChart,
        mergedLabels.invalidVotes,
        mergedLabels.nonVoters,
    ])

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

export const ParticipationSummary: React.FC<{
    result?: ResultsParticipationSummary | null
    chartName: string
    labels?: Partial<ResultsAndParticipationLabels>
    showWeight?: boolean
}> = ({result, chartName, labels, showWeight = false}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])

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
                                    <TableRow>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__label-cell"
                                            component="th"
                                            scope="row"
                                            sx={{overflowWrap: "anywhere"}}
                                        >
                                            {mergedLabels.eligibleCensus}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__value-cell"
                                            align="right"
                                        >
                                            {valueOrDash(result.eligibleCensus)}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__percent-cell"
                                            align="right"
                                        />
                                    </TableRow>
                                    <TableRow>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__label-cell"
                                            component="th"
                                            scope="row"
                                            sx={{overflowWrap: "anywhere"}}
                                        >
                                            {mergedLabels.totalAuditableVotes}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__value-cell"
                                            align="right"
                                        >
                                            {valueOrDash(result.totalAuditableVotes)}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__percent-cell"
                                            align="right"
                                        >
                                            {percentOrDash(result.totalAuditableVotesPercent)}
                                        </TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__label-cell"
                                            component="th"
                                            scope="row"
                                            sx={{overflowWrap: "anywhere"}}
                                        >
                                            {mergedLabels.totalVotesCounted}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__value-cell"
                                            align="right"
                                        >
                                            {valueOrDash(result.totalVotes)}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__percent-cell"
                                            align="right"
                                        >
                                            {percentOrDash(result.totalVotesPercent)}
                                        </TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__label-cell"
                                            component="th"
                                            scope="row"
                                            sx={{overflowWrap: "anywhere"}}
                                        >
                                            {mergedLabels.totalValidVotes}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__value-cell"
                                            align="right"
                                        >
                                            {valueOrDash(result.totalValidVotes)}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__percent-cell"
                                            align="right"
                                        >
                                            {percentOrDash(result.totalValidVotesPercent)}
                                        </TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__label-cell"
                                            component="th"
                                            scope="row"
                                            sx={{overflowWrap: "anywhere"}}
                                        >
                                            {mergedLabels.totalInvalidVotes}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__value-cell"
                                            align="right"
                                        >
                                            {valueOrDash(result.totalInvalidVotes)}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__percent-cell"
                                            align="right"
                                        >
                                            {percentOrDash(result.totalInvalidVotesPercent)}
                                        </TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__label-cell"
                                            component="th"
                                            scope="row"
                                            sx={{overflowWrap: "anywhere"}}
                                        >
                                            {mergedLabels.explicitInvalidVotes}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__value-cell"
                                            align="right"
                                        >
                                            {valueOrDash(result.explicitInvalidVotes)}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__percent-cell"
                                            align="right"
                                        >
                                            {percentOrDash(result.explicitInvalidVotesPercent)}
                                        </TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__label-cell"
                                            component="th"
                                            scope="row"
                                            sx={{overflowWrap: "anywhere"}}
                                        >
                                            {mergedLabels.implicitInvalidVotes}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__value-cell"
                                            align="right"
                                        >
                                            {valueOrDash(result.implicitInvalidVotes)}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__percent-cell"
                                            align="right"
                                        >
                                            {percentOrDash(result.implicitInvalidVotesPercent)}
                                        </TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__label-cell"
                                            component="th"
                                            scope="row"
                                            sx={{overflowWrap: "anywhere"}}
                                        >
                                            {mergedLabels.blankVotes}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__value-cell"
                                            align="right"
                                        >
                                            {valueOrDash(result.blankVotes)}
                                        </TableCell>
                                        <TableCell
                                            className="seq-tally-results-participation-summary__percent-cell"
                                            align="right"
                                        >
                                            {percentOrDash(result.blankVotesPercent)}
                                        </TableCell>
                                    </TableRow>
                                    {showWeight && (
                                        <TableRow>
                                            <TableCell
                                                className="seq-tally-results-participation-summary__label-cell"
                                                component="th"
                                                scope="row"
                                                sx={{overflowWrap: "anywhere"}}
                                            >
                                                {mergedLabels.weight}
                                            </TableCell>
                                            <TableCell
                                                className="seq-tally-results-participation-summary__value-cell"
                                                align="right"
                                            >
                                                {valueOrDash(result.weight)}
                                            </TableCell>
                                            <TableCell
                                                className="seq-tally-results-participation-summary__percent-cell"
                                                align="right"
                                            />
                                        </TableRow>
                                    )}
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

export const CandidateResultsChart: React.FC<{
    results: CandidateResultRow[]
    chartName: string
    labels?: Partial<ResultsAndParticipationLabels>
}> = ({results, chartName, labels}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const chartData = useMemo(() => {
        const representedResults = results
            .map((candidate) => ({
                label: candidate.name || "-",
                value: toFiniteNumber(candidate.castVotes) ?? 0,
            }))
            .filter((item) => item.value > 0)
            .sort((left, right) => right.value - left.value)

        if (representedResults.length > MAX_CANDIDATES_REPRESENTED) {
            const deletedItems = representedResults.splice(MAX_CANDIDATES_REPRESENTED)
            const othersSum = deletedItems.reduce((sum, item) => sum + item.value, 0)
            representedResults.push({label: mergedLabels.others, value: othersSum})
        }

        return representedResults
    }, [results, mergedLabels.others])

    const chartOptions = useMemo<ApexChartProps>(
        () => ({
            options: {
                labels: chartData.map((item) => item.label),
                legend: {
                    position: "right",
                },
                responsive: RESPONSIVE_PIE_OPTIONS,
                colors: ["#008FFBFF", "#FF0000", "#dfdf01ff", "#079107ff", "#FF8000", "#706565ff"],
            },
            series: chartData.map((item) => item.value),
        }),
        [chartData]
    )

    if (!results.length || !chartData.length) {
        return null
    }

    return (
        <ChartPanel title={chartName} className="seq-tally-results-candidate-chart">
            <Chart
                className="seq-tally-results-candidate-chart__chart"
                options={chartOptions.options}
                series={chartOptions.series}
                type="pie"
                width="100%"
                height={300}
            />
        </ChartPanel>
    )
}

export const CandidateResults: React.FC<{
    candidates: CandidateResultRow[]
    chartName: string
    labels?: Partial<ResultsAndParticipationLabels>
}> = ({candidates, chartName, labels}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const orderedCandidates = useMemo(
        () => [...candidates].sort(sortCandidateResults),
        [candidates]
    )
    const hasCandidateChartData = useMemo(
        () => orderedCandidates.some((candidate) => (toFiniteNumber(candidate.castVotes) ?? 0) > 0),
        [orderedCandidates]
    )
    const columns = useMemo<GridColDef<CandidateResultRow>[]>(
        () => [
            {
                field: "name",
                headerName: mergedLabels.options,
                flex: 1,
                minWidth: 180,
                editable: false,
                align: "left",
            },
            {
                field: "castVotes",
                headerName: mergedLabels.castVotes,
                flex: 1,
                minWidth: 140,
                editable: false,
                renderCell: (props: GridRenderCellParams<CandidateResultRow, NumericValue>) =>
                    valueOrDash(props.value),
                align: "right",
                headerAlign: "right",
            },
            {
                field: "castVotesPercent",
                headerName: mergedLabels.castVotesPercent,
                flex: 1,
                minWidth: 150,
                editable: false,
                renderCell: (props: GridRenderCellParams<CandidateResultRow, NumericValue>) =>
                    percentOrDash(props.value),
                align: "right",
                headerAlign: "right",
            },
            {
                field: "winningPosition",
                headerName: mergedLabels.winningPosition,
                flex: 1,
                minWidth: 150,
                editable: false,
                renderCell: (props: GridRenderCellParams<CandidateResultRow, NumericValue>) =>
                    valueOrDash(props.value),
                sortComparator: (left, right) => {
                    const maxInt = Number.MAX_SAFE_INTEGER
                    const leftPosition = toFiniteNumber(left) ?? maxInt
                    const rightPosition = toFiniteNumber(right) ?? maxInt

                    return leftPosition - rightPosition
                },
                align: "right",
                headerAlign: "right",
            },
        ],
        [
            mergedLabels.options,
            mergedLabels.castVotes,
            mergedLabels.castVotesPercent,
            mergedLabels.winningPosition,
        ]
    )
    const gridHeight = Math.min(Math.max(orderedCandidates.length * 52 + 116, 260), 680)

    return (
        <Box
            className="seq-tally-results-candidate-results"
            sx={{borderTop: "1px solid", borderColor: "divider", mt: 4, p: 0}}
        >
            <Typography
                className="seq-tally-results-candidate-results__title"
                variant="h6"
                component="div"
                sx={{mt: 6, ml: 1}}
            >
                {mergedLabels.candidateResults}
            </Typography>

            {orderedCandidates.length ? (
                <Box
                    className="seq-tally-results-candidate-results__content"
                    sx={{
                        display: "flex",
                        flexDirection: {xs: "column", lg: "row"},
                        gap: 4,
                        alignItems: "flex-start",
                    }}
                >
                    {hasCandidateChartData && (
                        <Box
                            className="seq-tally-results-candidate-results__chart-column"
                            sx={{
                                flex: {xs: "1 1 auto", lg: "0 0 450px"},
                                mt: 2,
                                width: {xs: "100%", lg: 450},
                                maxWidth: "100%",
                            }}
                        >
                            <CandidateResultsChart
                                results={orderedCandidates}
                                chartName={chartName}
                                labels={mergedLabels}
                            />
                        </Box>
                    )}
                    <Box
                        className="seq-tally-results-candidate-results__grid-column"
                        sx={{
                            flex: "1 1 auto",
                            alignItems: "center",
                            mt: 2,
                            minWidth: 0,
                            width: "100%",
                            height: gridHeight,
                        }}
                    >
                        <DataGrid
                            className="seq-tally-results-candidate-results__grid"
                            sx={{mt: 0}}
                            rows={orderedCandidates}
                            columns={columns}
                            getRowId={(row) => row.id}
                            initialState={DATA_GRID_INITIAL_STATE}
                            pageSizeOptions={DATA_GRID_PAGE_SIZE_OPTIONS}
                            disableRowSelectionOnClick
                        />
                    </Box>
                </Box>
            ) : (
                <Typography
                    className="seq-tally-results-candidate-results__empty"
                    color="text.secondary"
                    sx={{mt: 2}}
                >
                    {mergedLabels.empty}
                </Typography>
            )}
        </Box>
    )
}

export const PreferentialCandidateResults: React.FC<{
    processResults: PreferentialProcessResults
    candidates: CandidateResultRow[]
    labels?: Partial<ResultsAndParticipationLabels>
}> = ({processResults, candidates, labels}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const theme = useTheme()
    const isXL = useMediaQuery(theme.breakpoints.up("xl"))
    const isLarge = useMediaQuery(theme.breakpoints.up("lg"))
    const visibleRoundsCount = isXL ? 3 : isLarge ? 2 : 1
    const roundCount = processResults.rounds.length
    const initialVisibleRoundEnd = Math.min(visibleRoundsCount - 1, roundCount - 1)
    const [representedRounds, setRepresentedRounds] = useState({
        start: 0,
        end: initialVisibleRoundEnd,
    })
    const candidateById = useMemo(() => new Map(candidates.map((c) => [c.id, c])), [candidates])

    useEffect(() => {
        setRepresentedRounds((current) => {
            if (current.start === 0 && current.end === initialVisibleRoundEnd) {
                return current
            }

            return {
                start: 0,
                end: initialVisibleRoundEnd,
            }
        })
    }, [initialVisibleRoundEnd])

    if (!processResults.rounds.length) {
        return null
    }

    const {rounds, name_references} = processResults
    const visibleRounds = rounds.slice(representedRounds.start, representedRounds.end + 1)

    const getCandidateStatusInRound = (candidateId: string, roundIndex: number) => {
        const round = rounds[roundIndex]
        const isLastRound = roundIndex === rounds.length - 1

        if (round.winner?.id === candidateId) {
            return "winner"
        }

        if (isLastRound) {
            return "active"
        }

        if (round.eliminated_candidates?.some((candidate) => candidate.id === candidateId)) {
            return "eliminated"
        }

        for (let i = 0; i < roundIndex; i += 1) {
            if (
                rounds[i].eliminated_candidates?.some((candidate) => candidate.id === candidateId)
            ) {
                return "eliminated"
            }
        }

        return "active"
    }

    const handleNavigate = (direction: "left" | "right") => {
        if (direction === "right" && representedRounds.end < rounds.length - 1) {
            setRepresentedRounds({
                start: representedRounds.start + 1,
                end: representedRounds.end + 1,
            })
        } else if (direction === "left" && representedRounds.start > 0) {
            setRepresentedRounds({
                start: representedRounds.start - 1,
                end: representedRounds.end - 1,
            })
        }
    }

    return (
        <Box
            className="seq-tally-results-preferential-results"
            sx={{mt: 4, borderTop: "1px solid", borderColor: "divider", pt: 4}}
        >
            <TableContainer
                className="seq-tally-results-preferential-results__table-container"
                component={Paper}
                sx={{
                    maxWidth: "100%",
                    boxShadow: "none",
                    border: "1px solid",
                    borderColor: "divider",
                }}
            >
                <Table className="seq-tally-results-preferential-results__table">
                    <TableHead>
                        <TableRow>
                            <TableCell
                                className="seq-tally-results-preferential-results__candidate-heading"
                                sx={{
                                    position: "sticky",
                                    left: 0,
                                    backgroundColor: "#FBFBFB",
                                    zIndex: 3,
                                    fontWeight: 600,
                                    border: "1px solid #fff",
                                }}
                            >
                                {mergedLabels.candidate}
                            </TableCell>
                            {visibleRounds.map((_, visibleIndex) => {
                                const roundIndex = representedRounds.start + visibleIndex
                                const isFirstVisible = visibleIndex === 0
                                const isLastVisible = visibleIndex === visibleRounds.length - 1

                                return (
                                    <TableCell
                                        key={roundIndex}
                                        align="center"
                                        sx={{
                                            fontWeight: 600,
                                            width: 320,
                                            minWidth: 320,
                                            maxWidth: 320,
                                            whiteSpace: "nowrap",
                                            backgroundColor: "#FBFBFB",
                                            border: "1px solid #fff",
                                            position: "relative",
                                        }}
                                    >
                                        {isFirstVisible && representedRounds.start > 0 && (
                                            <Box
                                                className="seq-tally-results-preferential-results__previous-rounds-button"
                                                component="button"
                                                type="button"
                                                onClick={() => handleNavigate("left")}
                                                aria-label={mergedLabels.previousRounds}
                                                sx={{
                                                    position: "absolute",
                                                    left: 16,
                                                    top: "50%",
                                                    transform: "translateY(-50%)",
                                                    border: 0,
                                                    borderRadius: "50%",
                                                    backgroundColor: "#1e3a5f",
                                                    color: "white",
                                                    width: 24,
                                                    height: 24,
                                                    cursor: "pointer",
                                                }}
                                            >
                                                {"<"}
                                            </Box>
                                        )}
                                        <span className="seq-tally-results-preferential-results__round-label">
                                            {mergedLabels.round} {roundIndex + 1}
                                        </span>
                                        {isLastVisible &&
                                            representedRounds.end < rounds.length - 1 && (
                                                <Box
                                                    className="seq-tally-results-preferential-results__next-rounds-button"
                                                    component="button"
                                                    type="button"
                                                    onClick={() => handleNavigate("right")}
                                                    aria-label={mergedLabels.nextRounds}
                                                    sx={{
                                                        position: "absolute",
                                                        right: 16,
                                                        top: "50%",
                                                        transform: "translateY(-50%)",
                                                        border: 0,
                                                        borderRadius: "50%",
                                                        backgroundColor: "#1e3a5f",
                                                        color: "white",
                                                        width: 24,
                                                        height: 24,
                                                        cursor: "pointer",
                                                    }}
                                                >
                                                    {">"}
                                                </Box>
                                            )}
                                    </TableCell>
                                )
                            })}
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {name_references.map((candidate) => (
                            <TableRow
                                className="seq-tally-results-preferential-results__row"
                                key={candidate.id}
                            >
                                <TableCell
                                    className="seq-tally-results-preferential-results__candidate-cell"
                                    component="th"
                                    scope="row"
                                    sx={{
                                        position: "sticky",
                                        left: 0,
                                        backgroundColor: "#fff",
                                        zIndex: 2,
                                        border: "1px solid #fff",
                                        fontWeight: 500,
                                        maxWidth: 180,
                                        overflow: "hidden",
                                        textOverflow: "ellipsis",
                                        whiteSpace: "nowrap",
                                    }}
                                    title={candidateById.get(candidate.id)?.name ?? candidate.name}
                                >
                                    {candidateById.get(candidate.id)?.name ?? candidate.name}
                                </TableCell>
                                {visibleRounds.map((round, visibleIndex) => {
                                    const roundIndex = representedRounds.start + visibleIndex
                                    const status = getCandidateStatusInRound(
                                        candidate.id,
                                        roundIndex
                                    )
                                    const outcome = round.candidates_wins[candidate.id]

                                    return (
                                        <TableCell
                                            key={roundIndex}
                                            align="center"
                                            sx={{
                                                width: 320,
                                                minWidth: 320,
                                                maxWidth: 320,
                                                backgroundColor: outcome ? "#F9F9FF" : "#E0E0E0",
                                                border: "1px solid #fff",
                                            }}
                                        >
                                            {outcome ? (
                                                <Stack
                                                    className="seq-tally-results-preferential-results__round-outcome"
                                                    direction="row"
                                                    alignItems="center"
                                                    justifyContent="flex-start"
                                                    spacing={2}
                                                >
                                                    <Box
                                                        className="seq-tally-results-preferential-results__round-votes"
                                                        sx={{color: "#333", fontSize: "0.875rem"}}
                                                    >
                                                        {outcome.wins.toLocaleString("en-US")} (
                                                        {(outcome.percentage * 100).toFixed(2)}%)
                                                    </Box>
                                                    {status === "winner" && (
                                                        <Chip
                                                            className="seq-tally-results-preferential-results__winner-chip"
                                                            label={mergedLabels.winner}
                                                            sx={{
                                                                backgroundColor: "#4caf50",
                                                                color: "white",
                                                                fontWeight: 400,
                                                                fontSize: "0.875rem",
                                                            }}
                                                        />
                                                    )}
                                                    {status === "eliminated" && (
                                                        <Chip
                                                            className="seq-tally-results-preferential-results__eliminated-chip"
                                                            label={mergedLabels.eliminated}
                                                            variant="outlined"
                                                            sx={{
                                                                borderColor: "#f44336",
                                                                color: "#f44336",
                                                                fontWeight: 400,
                                                                fontSize: "0.875rem",
                                                            }}
                                                        />
                                                    )}
                                                </Stack>
                                            ) : null}
                                        </TableCell>
                                    )
                                })}
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </TableContainer>
        </Box>
    )
}

export const ResultsAndParticipation: React.FC<ResultsAndParticipationProps> = ({
    chartName,
    summary,
    candidates,
    labels,
    showWeight,
    processResults,
}) => (
    <Box className="seq-tally-results">
        <ParticipationSummary
            result={summary}
            chartName={chartName}
            labels={labels}
            showWeight={showWeight}
        />
        {processResults ? (
            <PreferentialCandidateResults
                processResults={processResults}
                candidates={candidates}
                labels={labels}
            />
        ) : (
            <CandidateResults candidates={candidates} chartName={chartName} labels={labels} />
        )}
    </Box>
)

export default ResultsAndParticipation
