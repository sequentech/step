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
import ReactApexChart, {Props as ApexChartProps} from "react-apexcharts"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {TALLY_RESULTS_PIE_HEIGHT, TALLY_RESULTS_PIE_PANEL_WIDTH} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {ResultsRow} from "@/types/results"
import {translatedLabel} from "@/services/resultLabels"

interface ResultsSummaryProps {
    elections: ResultsRow[]
    resultsElections: ResultsRow[]
    locale: string
}

const percent = (value: unknown): string => (isNumber(value) ? formatPercentOne(value) : "-")
const valueOrDash = (value: unknown): string | number =>
    typeof value === "string" || typeof value === "number" ? value : "-"
const stringOrUndefined = (value: unknown): string | undefined =>
    typeof value === "string" && value.length > 0 ? value : undefined
const sameId = (left: unknown, right: unknown): boolean =>
    left !== null &&
    left !== undefined &&
    right !== null &&
    right !== undefined &&
    String(left) === String(right)

const Chart = ((ReactApexChart as unknown as {default?: React.ComponentType<ApexChartProps>})
    .default ?? ReactApexChart) as React.ComponentType<ApexChartProps>

const finiteNumber = (value: unknown): number | null => {
    if (typeof value === "number") {
        return Number.isFinite(value) ? value : null
    }

    if (typeof value === "string" && value.trim().length > 0) {
        const parsed = Number(value)
        return Number.isFinite(parsed) ? parsed : null
    }

    return null
}

interface GeneralInformationChartProps {
    electionName: string
    result: ResultsRow
}

const GeneralInformationChart: React.FC<GeneralInformationChartProps> = ({
    electionName,
    result,
}) => {
    const {t} = useTranslation()
    const chartData = useMemo(() => {
        const eligibleCensus = finiteNumber(result.elegible_census)
        const totalVoters = finiteNumber(result.total_voters)

        if (eligibleCensus === null || totalVoters === null) {
            return []
        }

        const representedData = [
            {
                label: t("resultsPortal.summary.totalVotesCounted"),
                value: totalVoters,
            },
            {
                label: t("resultsPortal.resultsAndParticipation.nonVoters"),
                value: Math.max(eligibleCensus - totalVoters, 0),
            },
        ].filter((item) => item.value > 0)

        return representedData.length > 0
            ? representedData
            : [
                  {
                      label: t("resultsPortal.resultsAndParticipation.nonVoters"),
                      value: 100,
                  },
              ]
    }, [result.elegible_census, result.total_voters, t])
    const chartOptions = useMemo<ApexChartProps>(
        () => ({
            options: {
                labels: chartData.map((item) => item.label),
                legend: {position: "right"},
                responsive: [
                    {
                        breakpoint: 480,
                        options: {
                            chart: {width: 200},
                            legend: {position: "bottom"},
                        },
                    },
                ],
            },
            series: chartData.map((item) => item.value),
        }),
        [chartData]
    )

    if (chartData.length === 0) {
        return null
    }

    return (
        <Paper
            className="seq-results-summary__chart"
            data-election-id={String(result.election_id)}
            variant="outlined"
            sx={{p: 2, width: "100%", maxWidth: TALLY_RESULTS_PIE_PANEL_WIDTH}}
        >
            <Typography
                className="seq-results-summary__chart-title"
                variant="body1"
                sx={{color: "grey.600", fontSize: "16px", fontWeight: 400}}
            >
                {electionName}
            </Typography>
            <Box
                className="seq-results-summary__chart-body"
                sx={{borderTop: "1px solid", borderColor: "divider", mt: 2, pt: 2}}
            >
                <Chart
                    className="seq-results-summary__pie"
                    options={chartOptions.options}
                    series={chartOptions.series}
                    type="pie"
                    width="100%"
                    height={TALLY_RESULTS_PIE_HEIGHT}
                />
            </Box>
        </Paper>
    )
}

export const ResultsSummary: React.FC<ResultsSummaryProps> = ({
    elections,
    resultsElections,
    locale,
}) => {
    const {t} = useTranslation()

    return (
        <Box className="seq-results-summary" component="section" sx={{mt: {xs: 3, md: 5}}}>
            <Typography
                className="seq-results-summary__title"
                component="h2"
                variant="h5"
                sx={{mb: 2}}
            >
                {t("resultsPortal.summary.title")}
            </Typography>
            <Box
                className="seq-results-summary__content"
                sx={{
                    display: "flex",
                    flexDirection: {xs: "column", lg: "row"},
                    gap: 4,
                    alignItems: "flex-start",
                }}
            >
                <Box
                    className="seq-results-summary__charts"
                    sx={{
                        display: "flex",
                        flex: {
                            xs: "1 1 auto",
                            lg: `0 0 ${TALLY_RESULTS_PIE_PANEL_WIDTH}px`,
                        },
                        flexDirection: "column",
                        gap: 2,
                        width: {xs: "100%", lg: TALLY_RESULTS_PIE_PANEL_WIDTH},
                        maxWidth: "100%",
                    }}
                >
                    {resultsElections.map((result) => {
                        const election = elections.find((row) => sameId(row.id, result.election_id))
                        const electionName =
                            stringOrUndefined(result.name) ??
                            translatedLabel(election, locale, t("resultsPortal.summary.election"))

                        return (
                            <GeneralInformationChart
                                key={`${result.election_id}-${result.id}`}
                                electionName={electionName}
                                result={result}
                            />
                        )
                    })}
                </Box>
                <Box
                    className="seq-results-summary__table-column"
                    sx={{flex: "1 1 auto", minWidth: 0, width: "100%"}}
                >
                    <TableContainer
                        className="seq-results-summary__table-container"
                        sx={{
                            border: "1px solid",
                            borderColor: "divider",
                            borderRadius: 1,
                            overflowX: "auto",
                            bgcolor: "background.paper",
                        }}
                    >
                        <Table
                            className="seq-results-summary__table"
                            aria-label={t("resultsPortal.summary.ariaLabel")}
                            sx={{minWidth: 680}}
                        >
                            <TableHead className="seq-results-summary__table-head">
                                <TableRow className="seq-results-summary__heading-row">
                                    <TableCell className="seq-results-summary__election-heading">
                                        {t("resultsPortal.summary.election")}
                                    </TableCell>
                                    <TableCell
                                        className="seq-results-summary__eligible-heading"
                                        align="right"
                                    >
                                        {t("resultsPortal.summary.eligibleVoters")}
                                    </TableCell>
                                    <TableCell
                                        className="seq-results-summary__counted-heading"
                                        align="right"
                                    >
                                        {t("resultsPortal.summary.totalVotesCounted")}
                                    </TableCell>
                                    <TableCell
                                        className="seq-results-summary__participation-heading"
                                        align="right"
                                    >
                                        {t("resultsPortal.summary.participation")}
                                    </TableCell>
                                </TableRow>
                            </TableHead>
                            <TableBody className="seq-results-summary__table-body">
                                {resultsElections.map((result) => {
                                    const election = elections.find((row) =>
                                        sameId(row.id, result.election_id)
                                    )
                                    return (
                                        <TableRow
                                            className="seq-results-summary__row"
                                            key={`${result.election_id}-${result.id}`}
                                        >
                                            <TableCell className="seq-results-summary__election-cell">
                                                {stringOrUndefined(result.name) ??
                                                    translatedLabel(
                                                        election,
                                                        locale,
                                                        t("resultsPortal.summary.election")
                                                    )}
                                            </TableCell>
                                            <TableCell
                                                className="seq-results-summary__eligible-cell"
                                                align="right"
                                            >
                                                {valueOrDash(result.elegible_census)}
                                            </TableCell>
                                            <TableCell
                                                className="seq-results-summary__counted-cell"
                                                align="right"
                                            >
                                                {valueOrDash(result.total_voters)}
                                            </TableCell>
                                            <TableCell
                                                className="seq-results-summary__participation-cell"
                                                align="right"
                                            >
                                                {percent(result.total_voters_percent)}
                                            </TableCell>
                                        </TableRow>
                                    )
                                })}
                            </TableBody>
                        </Table>
                    </TableContainer>
                </Box>
            </Box>
        </Box>
    )
}
