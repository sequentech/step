// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useGetList, useGetOne} from "react-admin"

import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Election,
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Contest_Candidate,
} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams, GridComparatorFn} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"
import {NoItem} from "@/components/NoItem"
import {
    Typography,
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
import {useSQLQuery} from "@/hooks/useSQLiteDatabase"

interface TallyResultsGlobalCandidatesProps {
    contestId: string
    electionId: string
    electionEventId: string
    tenantId: string
    resultsEventId: string | null
    databaseBuffer: Uint8Array | null
}

// Define the comparator function
const winningPositionComparator: GridComparatorFn<string> = (v1, v2) => {
    const maxInt = Number.MAX_SAFE_INTEGER

    // Convert stringified numbers to integers, non-numeric strings to maxInt
    const pos1 = isNaN(parseInt(v1)) ? maxInt : parseInt(v1)
    const pos2 = isNaN(parseInt(v2)) ? maxInt : parseInt(v2)

    return pos1 - pos2
}

export const TallyResultsGlobalCandidates: React.FC<TallyResultsGlobalCandidatesProps> = (
    props
) => {
    const {contestId, electionId, electionEventId, tenantId, resultsEventId, databaseBuffer} = props
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const tallyData = useAtomValue(tallyQueryData)

    const [resultsData, setResultsData] = useState<Array<Sequent_Backend_Candidate_Extended>>([])

    const {data: candidates} = useSQLQuery(
        "SELECT * FROM candidate WHERE contest_id = ?",
        [contestId],
        {
            databaseBuffer: databaseBuffer,
            enabled: !!databaseBuffer && !!contestId,
        }
    )

    const {data: general} = useSQLQuery(
        "SELECT * FROM results_contest WHERE contest_id = ? and election_id = ?",
        [contestId, electionId],
        {
            databaseBuffer: databaseBuffer,
            enabled: !!databaseBuffer && !!contestId && !!electionId,
        }
    )

    const {data: results} = useSQLQuery(
        "SELECT * FROM results_contest_candidate WHERE contest_id = ? and election_id = ?",
        [contestId, electionId],
        {
            databaseBuffer: databaseBuffer,
            enabled: !!databaseBuffer && !!contestId && !!electionId,
        }
    )

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
                        election_event_id: electionEventId,
                        tenant_id: tenantId,
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
                                        ? formatPercentOne(general[0].total_auditable_votes_percent)
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
                                        ? formatPercentOne(general[0].total_invalid_votes_percent)
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
            ) : (
                <NoItem />
            )}

            <Typography variant="h6" component="div" sx={{mt: 8}}>
                {t("tally.table.candidates")}
            </Typography>

            {resultsData.length ? (
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
            ) : (
                <NoItem />
            )}
        </>
    )
}
