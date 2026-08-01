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
import {
    RESPONSIVE_PIE_OPTIONS,
    TALLY_RESULTS_PIE_HEIGHT,
    TALLY_RESULTS_PIE_PANEL_WIDTH,
} from "./constants"
import {Chart, ChartPanel} from "./ChartPanel"
import {
    PARTICIPATION_CHANNEL_ORDER,
    isKnownParticipationChannel,
    parseParticipationChannel,
    type KnownParticipationChannel,
    type ParticipationChannel,
} from "@sequentech/ui-core"
import type {
    ResultsAndParticipationLabelOverrides,
    ResultsAndParticipationLabels,
    ResultsParticipationSummary,
} from "./types"
import {mergeLabels, toFiniteNumber} from "./utils"

interface ParticipationByChannelProps {
    result: ResultsParticipationSummary
    chartName?: string
    labels?: ResultsAndParticipationLabelOverrides
}

const channelOrder = new Map<KnownParticipationChannel, number>(
    PARTICIPATION_CHANNEL_ORDER.map((channel, index) => [channel, index] as const)
)

const getChannelOrder = (channel: ParticipationChannel): number =>
    isKnownParticipationChannel(channel)
        ? (channelOrder.get(channel) ?? Number.MAX_SAFE_INTEGER)
        : Number.MAX_SAFE_INTEGER

const fallbackChannelLabel = (channel: ParticipationChannel): string =>
    channel
        .toLowerCase()
        .split("_")
        .filter(Boolean)
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join(" ") || channel

const channelLabel = (
    channel: ParticipationChannel,
    labels: ResultsAndParticipationLabels
): string =>
    isKnownParticipationChannel(channel)
        ? labels.channelNames[channel]
        : fallbackChannelLabel(channel)

const compareChannelKeys = (left: ParticipationChannel, right: ParticipationChannel): number => {
    if (left === right) return 0
    return left < right ? -1 : 1
}

const formatChannelPercentage = (total: number, totalChannelVotes: number): string => {
    if (totalChannelVotes <= 0) return "-"

    const percentage = Math.min(100, Math.max(0, (total / totalChannelVotes) * 100))
    return `${percentage.toFixed(1)}%`
}

export const ParticipationByChannel: React.FC<ParticipationByChannelProps> = ({
    result,
    chartName,
    labels,
}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const rows = useMemo(
        () =>
            Object.entries(result.votesByChannel ?? {})
                .map(([channel, value]) => ({
                    channel: parseParticipationChannel(channel),
                    total: toFiniteNumber(value),
                }))
                .filter(
                    (
                        row
                    ): row is {
                        channel: ParticipationChannel
                        total: number
                    } => row.total !== null && row.total > 0
                )
                .sort(
                    (left, right) =>
                        getChannelOrder(left.channel) - getChannelOrder(right.channel) ||
                        compareChannelKeys(left.channel, right.channel)
                ),
        [result.votesByChannel]
    )
    const totalChannelVotes = useMemo(
        () => rows.reduce((total, row) => total + row.total, 0),
        [rows]
    )
    const chartData = useMemo(
        () =>
            rows.map(({channel, total}) => ({
                label: channelLabel(channel, mergedLabels),
                value: total,
            })),
        [rows, mergedLabels]
    )
    const chartOptions = useMemo<ApexChartProps>(
        () => ({
            options: {
                labels: chartData.map((item) => item.label),
                legend: {position: "right"},
                responsive: RESPONSIVE_PIE_OPTIONS,
            },
            series: chartData.map((item) => item.value),
        }),
        [chartData]
    )

    if (rows.length === 0) return null

    return (
        <Box className="seq-tally-results-participation-by-channel" sx={{mt: 4}}>
            <Typography component="h3" variant="h6" sx={{mb: 2, ml: 1}}>
                {mergedLabels.participationByChannel}
            </Typography>
            <Box
                className="seq-tally-results-participation-by-channel__content"
                sx={{
                    display: "flex",
                    flexDirection: {xs: "column", lg: "row"},
                    gap: 4,
                    alignItems: "flex-start",
                }}
            >
                <Box
                    className="seq-tally-results-participation-by-channel__chart-column"
                    sx={{
                        flex: {xs: "1 1 auto", lg: `0 0 ${TALLY_RESULTS_PIE_PANEL_WIDTH}px`},
                        width: {xs: "100%", lg: TALLY_RESULTS_PIE_PANEL_WIDTH},
                        maxWidth: "100%",
                    }}
                >
                    <ChartPanel
                        title={chartName ?? mergedLabels.participationByChannel}
                        className="seq-tally-results-participation-by-channel-chart"
                    >
                        <Chart
                            className="seq-tally-results-participation-by-channel-chart__chart"
                            options={chartOptions.options}
                            series={chartOptions.series}
                            type="pie"
                            width="100%"
                            height={TALLY_RESULTS_PIE_HEIGHT}
                        />
                    </ChartPanel>
                </Box>
                <Box
                    className="seq-tally-results-participation-by-channel__table-column"
                    sx={{flex: "1 1 auto", minWidth: 0, width: "100%"}}
                >
                    <TableContainer
                        component={Paper}
                        sx={{border: "1px solid", borderColor: "divider"}}
                    >
                        <Table sx={{minWidth: {xs: 300, sm: 500}, tableLayout: "fixed"}}>
                            <TableHead>
                                <TableRow>
                                    <TableCell>{mergedLabels.channel}</TableCell>
                                    <TableCell align="right">{mergedLabels.total}</TableCell>
                                    <TableCell align="right">{mergedLabels.turnout}</TableCell>
                                </TableRow>
                            </TableHead>
                            <TableBody>
                                {rows.map(({channel, total}) => (
                                    <TableRow key={channel}>
                                        <TableCell component="th" scope="row">
                                            {channelLabel(channel, mergedLabels)}
                                        </TableCell>
                                        <TableCell align="right">{total}</TableCell>
                                        <TableCell align="right">
                                            {formatChannelPercentage(total, totalChannelVotes)}
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    </TableContainer>
                </Box>
            </Box>
        </Box>
    )
}
