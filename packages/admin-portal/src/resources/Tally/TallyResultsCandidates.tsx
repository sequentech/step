// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useGetList, useGetOne} from "react-admin"

import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Results_Area_Contest,
    Sequent_Backend_Results_Area_Contest_Candidate,
} from "../../gql/graphql"
import {useTranslation} from "react-i18next"
import {DataGrid, GridColDef, GridRenderCellParams, GridComparatorFn} from "@mui/x-data-grid"
import {NoItem} from "@/components/NoItem"
import {
    TableContainer,
    Box,
    Paper,
    Table,
    TableHead,
    TableRow,
    TableCell,
    TableBody,
    Typography,
} from "@mui/material"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Sequent_Backend_Candidate_Extended} from "./types"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {
    ParticipationSummaryChart,
    CandidatesResultsCharts,
} from "./TallyResultsGlobalCandidates"

interface TallyResultsCandidatesProps {
    areaId: string | null | undefined
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
export const TallyResultsCandidates: React.FC<TallyResultsCandidatesProps> = (props) => {
    const {areaId, contestId, electionId, electionEventId, tenantId, resultsEventId} = props
    const [resultsData, setResultsData] = useState<Array<Sequent_Backend_Candidate>>([])
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const tallyData = useAtomValue(tallyQueryData)

    const candidates: Array<Sequent_Backend_Candidate> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_candidate
                ?.filter((candidate) => contestId === candidate.contest_id)
                ?.map((candidate): Sequent_Backend_Candidate => candidate),
        [tallyData?.sequent_backend_candidate, contestId]
    )

    const general: Array<Sequent_Backend_Results_Area_Contest> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_results_area_contest?.filter(
                (areaContest) =>
                    contestId === areaContest.contest_id &&
                    electionId === areaContest.election_id &&
                    areaId === areaContest.area_id
            ),
        [tallyData?.sequent_backend_results_area_contest, contestId, electionId, areaId]
    )

    const results: Array<Sequent_Backend_Results_Area_Contest_Candidate> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_results_area_contest_candidate?.filter(
                (areaContestCandidate) =>
                    contestId === areaContestCandidate.contest_id &&
                    electionId === areaContestCandidate.election_id &&
                    areaId === areaContestCandidate.area_id
            ),
        [tallyData?.sequent_backend_results_area_contest_candidate, contestId, electionId]
    )

    const electionName: string | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_election?.find((election) => election.id === electionId)
                ?.name,
        [tallyData?.sequent_backend_election, electionId]
    )

    const contestName: string | undefined | null = useMemo(
        () => tallyData?.sequent_backend_contest?.find((contest) => contest.id === contestId)?.name,
        [tallyData?.sequent_backend_contest, contestId]
    )

    const areaName: string | undefined | null = useMemo(
        () => tallyData?.sequent_backend_area?.find((area) => area.id === areaId)?.name,
        [tallyData?.sequent_backend_area, areaId]
    )

    const getChartName = () => {
        if (electionName && contestName && areaName) {
            return `${electionName} - ${contestName} - ${areaName}`
        } else {
            return "-"
        }
    }

    useEffect(() => {
        if (results && candidates) {
            const temp: Array<Sequent_Backend_Candidate_Extended> | undefined = candidates?.map(
                (candidate, index) => {
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
            <Box sx={{ borderTop: '1px solid #ccc', mt: 4, p: 0 }}>
                <Typography variant="h6" component="div" sx={{mt: 6, ml: 1}}>
                    {t("tally.table.global")}
                </Typography>

                {general && general.length ? (
                    <Box sx={{display: "flex", flexDirection: "row", gap: 4, alignItems: "flex-start"}}>
                        {/* Chart on the left */}
                        <Box sx={{flex: "0 0 auto", mt: 2}}>
                            <ParticipationSummaryChart
                                result={general?.[0]}
                                chartName={getChartName()}
                            />
                        </Box>
                        {/* Table on the right */}
                        <Box sx={{flex: "1 1 auto", mt: 2}}>
                            <TableContainer component={Paper}>
                                <Table sx={{minWidth: 650}} aria-label="simple table">
                                    <TableHead>
                                        <TableRow>
                                            <TableCell></TableCell>
                                            <TableCell sx={{width: "25%"}} align="right">
                                                {t("tally.table.total")}
                                            </TableCell>
                                            <TableCell sx={{width: "25%"}} align="right" width="300px">
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
                        </Box>
                    </Box>
                ) : (
                    <NoItem />
                )}
            </Box>

            <Box sx={{ borderTop: '1px solid #ccc', mt: 4, p: 0 }}>
                <Typography variant="h6" component="div" sx={{mt: 6, ml: 1}}>
                    {t("tally.table.candidates")}
                </Typography>

                {resultsData.length ? (
                    <Box sx={{display: "flex", flexDirection: "row", gap: 4, alignItems: "flex-start"}}>
                        <Box sx={{flex: "0 0 auto", mt: 2}}>
                            <CandidatesResultsCharts
                                results={resultsData as Sequent_Backend_Candidate_Extended[]}
                                chartName={getChartName()}
                            />
                        </Box>
                        <Box sx={{flex: "1 1 auto", mt: 2}}>
                            <DataGrid
                                sx={{mt: 0}}
                                rows={resultsData}
                                columns={columns}
                                initialState={{
                                    pagination: {
                                        paginationModel: {
                                            pageSize: 10,
                                        },
                                    },
                                    sorting: {
                                        sortModel: [{field: "winning_position", sort: "asc"}],
                                    },
                                }}
                                pageSizeOptions={[10, 20, 50, 100]}
                                disableRowSelectionOnClick
                            />
                        </Box>
                    </Box>
                ) : (
                    <NoItem />
                )}
            </Box>
                
        </>
    )
}
