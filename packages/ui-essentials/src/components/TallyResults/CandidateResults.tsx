// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo} from "react"
import {Box, Typography} from "@mui/material"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import type {Props as ApexChartProps} from "react-apexcharts"
import {
    CANDIDATE_CHART_COLORS,
    DATA_GRID_INITIAL_STATE,
    DATA_GRID_PAGE_SIZE_OPTIONS,
    RESPONSIVE_PIE_OPTIONS,
    TALLY_RESULTS_PIE_HEIGHT,
    TALLY_RESULTS_PIE_PANEL_WIDTH,
} from "./constants"
import {Chart, ChartPanel} from "./ChartPanel"
import type {
    CandidateResultRow,
    NumericValue,
    ResultsAndParticipationLabelOverrides,
    ResultsAndParticipationLabels,
} from "./types"
import {
    buildCandidateChartData,
    mergeLabels,
    percentOrDash,
    toFiniteNumber,
    valueOrDash,
} from "./utils"

interface CandidateResultsChartProps {
    results: CandidateResultRow[]
    chartName: string
    labels?: ResultsAndParticipationLabelOverrides
}

interface CandidateResultsProps {
    candidates: CandidateResultRow[]
    chartName: string
    labels?: ResultsAndParticipationLabelOverrides
}

const winningPositionComparator = (left: NumericValue, right: NumericValue) => {
    const maxInt = Number.MAX_SAFE_INTEGER
    const leftPosition = toFiniteNumber(left) ?? maxInt
    const rightPosition = toFiniteNumber(right) ?? maxInt

    return leftPosition - rightPosition
}

const classToken = (value: string | number) => String(value).replace(/[^a-zA-Z0-9_-]/g, "-")

const useCandidateResultColumns = (
    labels: ResultsAndParticipationLabels
): GridColDef<CandidateResultRow>[] =>
    useMemo(
        () => [
            {
                field: "name",
                headerName: labels.options,
                flex: 1,
                minWidth: 180,
                editable: false,
                align: "left",
                headerClassName: "seq-tally-results-candidate-results__name-heading",
                cellClassName: "seq-tally-results-candidate-results__name-cell",
            },
            {
                field: "castVotes",
                headerName: labels.castVotes,
                flex: 1,
                minWidth: 140,
                editable: false,
                renderCell: (props: GridRenderCellParams<CandidateResultRow, NumericValue>) =>
                    valueOrDash(props.value),
                align: "right",
                headerAlign: "right",
                headerClassName: "seq-tally-results-candidate-results__votes-heading",
                cellClassName: "seq-tally-results-candidate-results__votes-cell",
            },
            {
                field: "castVotesPercent",
                headerName: labels.castVotesPercent,
                flex: 1,
                minWidth: 150,
                editable: false,
                renderCell: (props: GridRenderCellParams<CandidateResultRow, NumericValue>) =>
                    percentOrDash(props.value),
                align: "right",
                headerAlign: "right",
                headerClassName: "seq-tally-results-candidate-results__percent-heading",
                cellClassName: "seq-tally-results-candidate-results__percent-cell",
            },
            {
                field: "winningPosition",
                headerName: labels.winningPosition,
                flex: 1,
                minWidth: 150,
                editable: false,
                renderCell: (props: GridRenderCellParams<CandidateResultRow, NumericValue>) =>
                    valueOrDash(props.value),
                sortComparator: winningPositionComparator,
                align: "right",
                headerAlign: "right",
                headerClassName: "seq-tally-results-candidate-results__position-heading",
                cellClassName: "seq-tally-results-candidate-results__position-cell",
            },
        ],
        [labels.options, labels.castVotes, labels.castVotesPercent, labels.winningPosition]
    )

export const CandidateResultsChart: React.FC<CandidateResultsChartProps> = ({
    results,
    chartName,
    labels,
}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const chartData = useMemo(
        () => buildCandidateChartData(results, mergedLabels),
        [results, mergedLabels]
    )
    const chartOptions = useMemo<ApexChartProps>(
        () => ({
            options: {
                labels: chartData.map((item) => item.label),
                legend: {
                    position: "right",
                },
                responsive: RESPONSIVE_PIE_OPTIONS,
                colors: CANDIDATE_CHART_COLORS,
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
                height={TALLY_RESULTS_PIE_HEIGHT}
            />
        </ChartPanel>
    )
}

export const CandidateResults: React.FC<CandidateResultsProps> = ({
    candidates,
    chartName,
    labels,
}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const hasCandidateChartData = useMemo(
        () => candidates.some((candidate) => (toFiniteNumber(candidate.castVotes) ?? 0) > 0),
        [candidates]
    )
    const columns = useCandidateResultColumns(mergedLabels)
    const gridHeight = Math.min(Math.max(candidates.length * 52 + 116, 260), 680)

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

            {candidates.length ? (
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
                                flex: {
                                    xs: "1 1 auto",
                                    lg: `0 0 ${TALLY_RESULTS_PIE_PANEL_WIDTH}px`,
                                },
                                mt: 2,
                                width: {xs: "100%", lg: TALLY_RESULTS_PIE_PANEL_WIDTH},
                                maxWidth: "100%",
                            }}
                        >
                            <CandidateResultsChart
                                results={candidates}
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
                            rows={candidates}
                            columns={columns}
                            getRowId={(row) => row.id}
                            getRowClassName={({id}) =>
                                `seq-tally-results-candidate-results__row seq-tally-results-candidate-results__row--${classToken(
                                    id
                                )}`
                            }
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
