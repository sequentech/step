// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"

import {useTranslation} from "react-i18next"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {NoItem} from "@/components/NoItem"
import {
    TableContainer,
    Paper,
    Table,
    TableHead,
    TableRow,
    TableCell,
    TableBody,
    Typography,
} from "@mui/material"
import {formatPercentOne, isNumber} from "@sequentech/ui-essentials"
import {useAtom} from "jotai"
import {tallyAreaCandidates, tallyAreaData} from "@/atoms/tally-candidates"


export const TallyResultsCandidates: React.FC = () => {
    const {t} = useTranslation()

    const [areaData] = useAtom(tallyAreaData)
    const [resultsData] = useAtom(tallyAreaCandidates)

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
            align: "right",
            headerAlign: "right",
        },
    ]

    return (
        <>
            <Typography variant="h6" component="div" sx={{mt: 8}}>
                {t("tally.table.global")}
            </Typography>

            {areaData ? (
                <TableContainer component={Paper}>
                    <Table sx={{minWidth: 650}} aria-label="simple table">
                        <TableHead>
                            <TableRow>
                                <TableCell></TableCell>
                                <TableCell align="right">{t("tally.table.total")}</TableCell>
                                <TableCell align="right">{t("tally.table.turnout")}</TableCell>
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                <TableCell component="th" scope="row">
                                    {t("tally.table.elegible_census")}
                                </TableCell>
                                <TableCell align="right">
                                    {areaData.elegible_census ?? "-"}
                                </TableCell>
                                <TableCell align="right"></TableCell>
                            </TableRow>
                            <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                <TableCell component="th" scope="row">
                                    {t("tally.table.total_votes")}
                                </TableCell>
                                <TableCell align="right">
                                    {areaData.total_votes ?? "-"}
                                </TableCell>
                                <TableCell align="right">
                                    {isNumber(areaData.total_votes_percent)
                                        ? formatPercentOne(areaData.total_votes_percent)
                                        : "-"}
                                </TableCell>
                            </TableRow>
                            <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                <TableCell component="th" scope="row">
                                    {t("tally.table.total_valid_votes")}
                                </TableCell>
                                <TableCell align="right">
                                    {areaData.total_valid_votes ?? "-"}
                                </TableCell>
                                <TableCell align="right">
                                    {isNumber(areaData.total_valid_votes_percent)
                                        ? formatPercentOne(areaData.total_valid_votes_percent)
                                        : "-"}
                                </TableCell>
                            </TableRow>
                            <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                <TableCell component="th" scope="row">
                                    {t("tally.table.total_invalid_votes")}
                                </TableCell>
                                <TableCell align="right">
                                    {areaData.total_invalid_votes ?? "-"}
                                </TableCell>
                                <TableCell align="right">
                                    {isNumber(areaData.total_invalid_votes_percent)
                                        ? formatPercentOne(areaData.total_invalid_votes_percent)
                                        : "-"}
                                </TableCell>
                            </TableRow>
                            <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                <TableCell component="th" scope="row">
                                    {t("tally.table.explicit_invalid_votes")}
                                </TableCell>
                                <TableCell align="right">
                                    {areaData.explicit_invalid_votes ?? "-"}
                                </TableCell>
                                <TableCell align="right">
                                    {isNumber(areaData.explicit_invalid_votes_percent)
                                        ? formatPercentOne(
                                              areaData.explicit_invalid_votes_percent
                                          )
                                        : "-"}
                                </TableCell>
                            </TableRow>
                            <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                <TableCell component="th" scope="row">
                                    {t("tally.table.implicit_invalid_votes")}
                                </TableCell>
                                <TableCell align="right">
                                    {areaData.implicit_invalid_votes ?? "-"}
                                </TableCell>
                                <TableCell align="right">
                                    {isNumber(areaData.implicit_invalid_votes_percent)
                                        ? formatPercentOne(
                                              areaData.implicit_invalid_votes_percent
                                          )
                                        : "-"}
                                </TableCell>
                            </TableRow>
                            <TableRow sx={{"&:last-child td, &:last-child th": {border: 0}}}>
                                <TableCell component="th" scope="row">
                                    {t("tally.table.blank_votes")}
                                </TableCell>
                                <TableCell align="right">
                                    {areaData.blank_votes ?? "-"}
                                </TableCell>
                                <TableCell align="right">
                                    {isNumber(areaData.blank_votes_percent)
                                        ? formatPercentOne(areaData.blank_votes_percent)
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
                                pageSize: 10,
                            },
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
