// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useGetList, useGetOne} from "react-admin"
import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Result,
    Sequent_Backend_Results_Area_Contest,
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Contest_Candidate,
} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams, GridComparatorFn} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"
import {NoItem} from "@/components/NoItem"
import {
    Typography,
    Box,
    TableContainer,
    Paper,
    Table,
    TableBody,
    TableRow,
    TableCell,
    TableHead,
} from "@mui/material"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Sequent_Backend_Candidate_Extended} from "./types"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"
import Chart, {Props} from "react-apexcharts"
import CardChart from "@/components/dashboard/charts/Charts"
import Item from "antd/es/list/Item"

interface TallyResultsGlobalCandidatesProps {
    contestId: string
    electionId: string
    electionEventId: string
    tenantId: string
    resultsEventId: string | null
}

// Define the comparator function
const winningPositionComparator: GridComparatorFn<string> = (v1, v2) => {
    const maxInt = Number.MAX_SAFE_INTEGER

    // Convert stringified numbers to integers, non-numeric strings to maxInt
    const pos1 = isNaN(parseInt(v1)) ? maxInt : parseInt(v1)
    const pos2 = isNaN(parseInt(v2)) ? maxInt : parseInt(v2)

    return pos1 - pos2
}

interface ResultsAndParticipationChartsProps {
    result: Sequent_Backend_Results_Contest | Sequent_Backend_Results_Area_Contest
    chartName: string
}

export const ResultsAndParticipationCharts: React.FC<ResultsAndParticipationChartsProps> = ({
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
                position: "bottom",
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
            sx={{display: "flex", flexDirection: "row", alignItems: "left", mb: 2}}
        >
            <CardChart title={chartName}>
                <Chart
                    options={chartOptions.options}
                    series={chartOptions.series}
                    type="pie"
                    width={400}
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

    const chartData = [
        ...results.map((candidate) => {
            return {
                label: candidate.name ?? "-",
                value: (candidate.cast_votes ?? 0) as number,
            }
        }),
    ].filter((item) => item.value && item.value > 0)

    const totalCandidatesRepresented = chartData ? chartData?.length : 0
    console.log(totalCandidatesRepresented)
    if (totalCandidatesRepresented === 0) {
        return null
    }

    // Distribute numbers evenly across the range based on index to have a different color for each candidate
    let colorsArray = chartData.map((_, index) => {
        let pad = index.toString(16)[0]
        return `#${Math.floor((index / totalCandidatesRepresented) * 16777215).toString(16).padStart(8, pad)}`
    }) as any[]
    colorsArray[0] = "#000000ff" // Set the first color to black

    const chartOptions: Props = {
        options: {
            labels: chartData.map((item) => item.label),
            legend: {
                position: "bottom",
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
            colors: colorsArray,
        },
        series: chartData.map((item) => item.value),
    }

    return (
        <Box
            key={chartName + "-candidates"}
            sx={{display: "flex", flexDirection: "row", alignItems: "left", mb: 2}}
        >
            <CardChart title={chartName}>
                <Chart
                    options={chartOptions.options}
                    series={chartOptions.series}
                    type="pie"
                    width={600}
                    height={500}
                />
            </CardChart>
        </Box>
    )
}

export const TallyResultsGlobalCandidates: React.FC<TallyResultsGlobalCandidatesProps> = (
    props
) => {
    const {contestId, electionId, electionEventId, tenantId, resultsEventId} = props
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const tallyData = useAtomValue(tallyQueryData)

    const [resultsData, setResultsData] = useState<Array<Sequent_Backend_Candidate_Extended>>([])

    const candidates: Array<Sequent_Backend_Candidate> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_candidate?.filter(
                (candidate) => contestId === candidate.contest_id
            ),
        [tallyData?.sequent_backend_candidate, contestId]
    )

    const general: Array<Sequent_Backend_Results_Contest> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_results_contest?.filter(
                (resultsContest) =>
                    contestId === resultsContest.contest_id &&
                    electionId === resultsContest.election_id
            ),
        [tallyData?.sequent_backend_results_contest, contestId, electionId]
    )

    const results: Array<Sequent_Backend_Results_Contest_Candidate> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_results_contest_candidate?.filter(
                (resultsContestCandidate) =>
                    contestId === resultsContestCandidate.contest_id &&
                    electionId === resultsContestCandidate.election_id
            ),
        [tallyData?.sequent_backend_results_contest_candidate, contestId, electionId]
    )

    const electionName: string | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_election?.find((election) => election.id === electionId)
                ?.name,
        [tallyData?.sequent_backend_election, electionId]
    )

    const getChartName = (contestName: string | undefined) => {
        if (electionName && contestName) {
            return `${electionName} - ${contestName} - ` + t("tally.common.global")
        } else {
            return "-"
        }
    }

    useEffect(() => {
        if (results && candidates) {
            const temp: Array<Sequent_Backend_Candidate_Extended> | undefined = candidates?.map(
                (candidate, index): Sequent_Backend_Candidate_Extended => {
                    let candidateResult = results.find((r) => r.candidate_id === candidate.id)

                    return {
                        ...candidate,
                        rowId: index,
                        id: candidate.id || "",
                        name: candidate.name,
                        status: "",
                        cast_votes: candidateResult?.cast_votes,
                        cast_votes_percent: candidateResult?.cast_votes_percent,
                        winning_position: candidateResult?.winning_position,
                    }
                }
            )

            setResultsData(temp)
        }
    }, [results, candidates])

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: t("tally.table.options"),
            flex: 1,
            editable: false,
            align: "left",
        },
        {
            field: "cast_votes",
            headerName: t("tally.table.cast_votes"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) => props["value"] ?? "-",
            align: "right",
            headerAlign: "right",
        },
        {
            field: "cast_votes_percent",
            headerName: t("tally.table.cast_votes_percent"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) =>
                isNumber(props["value"]) ? formatPercentOne(props["value"]) : "-",
            align: "right",
            headerAlign: "right",
        },
        {
            field: "winning_position",
            headerName: t("tally.table.winning_position"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => props["value"] ?? "-",
            sortComparator: winningPositionComparator,
            align: "right",
            headerAlign: "right",
        },
    ]

    return (
        <>
            <Typography variant="h6" component="div" sx={{mt: 8}}>
                {t("tally.table.global")}
            </Typography>

            {general && general.length ? (
                <>
                    <TableContainer component={Paper}>
                        <Table sx={{minWidth: 650}} aria-label="simple table">
                            <TableHead>
                                <TableRow>
                                    <TableCell></TableCell>
                                    <TableCell sx={{width: "25%"}} align="right">
                                        {t("tally.table.total")}
                                    </TableCell>
                                    <TableCell sx={{width: "25%"}} align="right">
                                        {t("tally.table.turnout")}
                                    </TableCell>
                                </TableRow>
                            </TableHead>
                            <TableBody>
                                <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                    <TableCell component="th" scope="row">
                                        {t("tally.table.elegible_census")}
                                    </TableCell>
                                    <TableCell align="right">
                                        {general?.[0].elegible_census ?? "-"}
                                    </TableCell>
                                    <TableCell align="right"></TableCell>
                                </TableRow>
                                <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                    <TableCell component="th" scope="row">
                                        {t("tally.table.total_auditable_votes")}
                                    </TableCell>
                                    <TableCell align="right">
                                        {general?.[0].total_auditable_votes ?? "-"}
                                    </TableCell>
                                    <TableCell align="right">
                                        {isNumber(general?.[0].total_auditable_votes_percent)
                                            ? formatPercentOne(
                                                  general[0].total_auditable_votes_percent
                                              )
                                            : "-"}
                                    </TableCell>
                                </TableRow>
                                <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                    <TableCell component="th" scope="row">
                                        {t("tally.table.total_votes_counted")}
                                    </TableCell>
                                    <TableCell align="right">
                                        {general?.[0].total_votes ?? "-"}
                                    </TableCell>
                                    <TableCell align="right">
                                        {isNumber(general?.[0].total_votes_percent)
                                            ? formatPercentOne(general[0].total_votes_percent)
                                            : "-"}
                                    </TableCell>
                                </TableRow>
                                <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                    <TableCell component="th" scope="row">
                                        {t("tally.table.total_valid_votes")}
                                    </TableCell>
                                    <TableCell align="right">
                                        {general?.[0].total_valid_votes ?? "-"}
                                    </TableCell>
                                    <TableCell align="right">
                                        {isNumber(general?.[0].total_valid_votes_percent)
                                            ? formatPercentOne(general[0].total_valid_votes_percent)
                                            : "-"}
                                    </TableCell>
                                </TableRow>
                                <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                    <TableCell component="th" scope="row">
                                        {t("tally.table.total_invalid_votes")}
                                    </TableCell>
                                    <TableCell align="right">
                                        {general?.[0].total_invalid_votes ?? "-"}
                                    </TableCell>
                                    <TableCell align="right">
                                        {isNumber(general?.[0].total_invalid_votes_percent)
                                            ? formatPercentOne(
                                                  general[0].total_invalid_votes_percent
                                              )
                                            : "-"}
                                    </TableCell>
                                </TableRow>
                                <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                    <TableCell component="th" scope="row">
                                        {t("tally.table.explicit_invalid_votes")}
                                    </TableCell>
                                    <TableCell align="right">
                                        {general?.[0].explicit_invalid_votes ?? "-"}
                                    </TableCell>
                                    <TableCell align="right">
                                        {isNumber(general?.[0].explicit_invalid_votes_percent)
                                            ? formatPercentOne(
                                                  general[0].explicit_invalid_votes_percent
                                              )
                                            : "-"}
                                    </TableCell>
                                </TableRow>
                                <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                    <TableCell component="th" scope="row">
                                        {t("tally.table.implicit_invalid_votes")}
                                    </TableCell>
                                    <TableCell align="right">
                                        {general?.[0].implicit_invalid_votes ?? "-"}
                                    </TableCell>
                                    <TableCell align="right">
                                        {isNumber(general?.[0].implicit_invalid_votes_percent)
                                            ? formatPercentOne(
                                                  general[0].implicit_invalid_votes_percent
                                              )
                                            : "-"}
                                    </TableCell>
                                </TableRow>
                                <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                    <TableCell component="th" scope="row">
                                        {t("tally.table.blank_votes")}
                                    </TableCell>
                                    <TableCell align="right">
                                        {general?.[0].blank_votes ?? "-"}
                                    </TableCell>
                                    <TableCell align="right">
                                        {isNumber(general?.[0].blank_votes_percent)
                                            ? formatPercentOne(general[0].blank_votes_percent)
                                            : "-"}
                                    </TableCell>
                                </TableRow>
                            </TableBody>
                        </Table>
                    </TableContainer>
                    <Box sx={{mt: 8}}>
                        <ResultsAndParticipationCharts
                            result={general?.[0]}
                            chartName={getChartName(general?.[0].name ?? undefined)}
                        />
                    </Box>
                </>
            ) : (
                <NoItem />
            )}

            <Typography variant="h6" component="div" sx={{mt: 8}}>
                {t("tally.table.candidates")}
            </Typography>

            {resultsData.length ? (
                <>
                    <DataGrid
                        rows={resultsData}
                        columns={columns}
                        initialState={{
                            pagination: {
                                paginationModel: {
                                    pageSize: 20,
                                },
                            },
                            sorting: {
                                sortModel: [{field: "winning_position", sort: "asc"}],
                            },
                        }}
                        pageSizeOptions={[10, 20, 50, 100]}
                        disableRowSelectionOnClick
                    />
                    <Box sx={{mt: 8}}>
                        <CandidatesResultsCharts
                            results={resultsData}
                            chartName={getChartName(general?.[0].name ?? undefined)}
                        />
                    </Box>
                </>
            ) : (
                <NoItem />
            )}
        </>
    )
}
