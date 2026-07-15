// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {
    Box,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
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
    )
}
