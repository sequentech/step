// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
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
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {ParticipationSummaryChart} from "./TallyResultsCharts"
import {
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Area_Contest,
} from "../../gql/graphql"

interface TallyResultsSummaryProps {
    general:
        | Array<Sequent_Backend_Results_Contest | Sequent_Backend_Results_Area_Contest>
        | undefined
    chartName: string
    showWeight?: boolean
    weight?: number | null
}

export const TallyResultsSummary: React.FC<TallyResultsSummaryProps> = ({
    general,
    chartName,
    showWeight = false,
    weight = null,
}) => {
    const {t} = useTranslation()
    const summaryRowClassName = (rowClassName: string) =>
        `participation-summary-row ${rowClassName}`
    const summaryRowSx = {"&:last-child td, &:last-child th": {border: 0}}

    return (
        <Box sx={{borderTop: "1px solid #ccc", mt: 4, p: 0}}>
            <Typography variant="h6" component="div" sx={{mt: 6, ml: 1}}>
                {t("tally.table.global")}
            </Typography>

            {general && general.length ? (
                <Box
                    sx={{
                        display: "flex",
                        flexDirection: {xs: "column", lg: "row"},
                        gap: 4,
                        alignItems: "flex-start",
                    }}
                >
                    <Box sx={{flex: {xs: "1 1 auto", lg: "0 0 auto"}, mt: 2}}>
                        <ParticipationSummaryChart result={general?.[0]} chartName={chartName} />
                    </Box>
                    <Box
                        sx={{
                            flex: "1 1 auto",
                            mt: 2,
                            border: "1px solid #cccccc99",
                            minWidth: 0,
                        }}
                    >
                        <TableContainer component={Paper}>
                            <Table
                                className="participation-summary-table"
                                sx={{minWidth: {xs: 300, sm: 650}}}
                                aria-label="simple table"
                            >
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
                                    <TableRow
                                        className={summaryRowClassName("eligible-voters")}
                                        sx={summaryRowSx}
                                    >
                                        <TableCell component="th" scope="row">
                                            {t("tally.table.elegible_census")}
                                        </TableCell>
                                        <TableCell align="right">
                                            {general?.[0].elegible_census ?? "-"}
                                        </TableCell>
                                        <TableCell align="right"></TableCell>
                                    </TableRow>
                                    <TableRow
                                        className={summaryRowClassName("total-auditable-votes")}
                                        sx={summaryRowSx}
                                    >
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
                                    <TableRow
                                        className={summaryRowClassName("total-votes-counted")}
                                        sx={summaryRowSx}
                                    >
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
                                    <TableRow
                                        className={summaryRowClassName("total-valid-votes")}
                                        sx={summaryRowSx}
                                    >
                                        <TableCell component="th" scope="row">
                                            {t("tally.table.total_valid_votes")}
                                        </TableCell>
                                        <TableCell align="right">
                                            {general?.[0].total_valid_votes ?? "-"}
                                        </TableCell>
                                        <TableCell align="right">
                                            {isNumber(general?.[0].total_valid_votes_percent)
                                                ? formatPercentOne(
                                                      general[0].total_valid_votes_percent
                                                  )
                                                : "-"}
                                        </TableCell>
                                    </TableRow>
                                    <TableRow
                                        className={summaryRowClassName("total-invalid-votes")}
                                        sx={summaryRowSx}
                                    >
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
                                    <TableRow
                                        className={summaryRowClassName("explicitly-invalid-votes")}
                                        sx={summaryRowSx}
                                    >
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
                                    <TableRow
                                        className={summaryRowClassName("implicitly-invalid-votes")}
                                        sx={summaryRowSx}
                                    >
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
                                    <TableRow
                                        className={summaryRowClassName("blank-votes")}
                                        sx={summaryRowSx}
                                    >
                                        <TableCell component="th" scope="row">
                                            {t("tally.table.blank_votes")}
                                        </TableCell>
                                        <TableCell align="right">
                                            {general?.[0].total_blank_votes ?? "-"}
                                        </TableCell>
                                        <TableCell align="right">
                                            {isNumber(general?.[0].total_blank_votes_percent)
                                                ? formatPercentOne(
                                                      general[0].total_blank_votes_percent
                                                  )
                                                : "-"}
                                        </TableCell>
                                    </TableRow>
                                    <TableRow
                                        sx={{"&:last-child td, &:last-child th": {border: 0}}}
                                    >
                                        <TableCell component="th" scope="row">
                                            {t("tally.table.explicit_blank_votes")}
                                        </TableCell>
                                        <TableCell align="right">
                                            {general?.[0].explicit_blank_votes ?? "-"}
                                        </TableCell>
                                        <TableCell align="right">
                                            {isNumber(general?.[0].explicit_blank_votes_percent)
                                                ? formatPercentOne(
                                                      general[0].explicit_blank_votes_percent
                                                  )
                                                : "-"}
                                        </TableCell>
                                    </TableRow>
                                    <TableRow
                                        sx={{"&:last-child td, &:last-child th": {border: 0}}}
                                    >
                                        <TableCell component="th" scope="row">
                                            {t("tally.table.implicit_blank_votes")}
                                        </TableCell>
                                        <TableCell align="right">
                                            {general?.[0].implicit_blank_votes ?? "-"}
                                        </TableCell>
                                        <TableCell align="right">
                                            {isNumber(general?.[0].implicit_blank_votes_percent)
                                                ? formatPercentOne(
                                                      general[0].implicit_blank_votes_percent
                                                  )
                                                : "-"}
                                        </TableCell>
                                    </TableRow>
                                    {showWeight && (
                                        <TableRow
                                            className={summaryRowClassName("weight")}
                                            sx={summaryRowSx}
                                        >
                                            <TableCell component="th" scope="row">
                                                {t("tally.table.weight")}
                                            </TableCell>
                                            <TableCell align="right">{weight ?? "-"}</TableCell>
                                            <TableCell align="right"></TableCell>
                                        </TableRow>
                                    )}
                                </TableBody>
                            </Table>
                        </TableContainer>
                    </Box>
                </Box>
            ) : (
                <NoItem />
            )}
        </Box>
    )
}
