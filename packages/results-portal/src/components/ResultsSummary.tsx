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
import {ResultsRow} from "@/types/results"
import {translatedLabel} from "@/services/resultLabels"

interface ResultsSummaryProps {
    elections: ResultsRow[]
    resultsElections: ResultsRow[]
    locale: string
}

const percent = (value: unknown): string => (isNumber(value) ? formatPercentOne(value) : "-")
const valueOrDash = (value: unknown): string | number => value ?? "-"

export const ResultsSummary: React.FC<ResultsSummaryProps> = ({
    elections,
    resultsElections,
    locale,
}) => (
    <Box component="section" sx={{mt: {xs: 3, md: 5}}}>
        <Typography component="h2" variant="h5" sx={{mb: 2}}>
            General information
        </Typography>
        <TableContainer
            sx={{
                border: "1px solid",
                borderColor: "divider",
                borderRadius: 1,
                overflowX: "auto",
                bgcolor: "background.paper",
            }}
        >
            <Table aria-label="General results information" sx={{minWidth: 680}}>
                <TableHead>
                    <TableRow>
                        <TableCell>Election</TableCell>
                        <TableCell align="right">Eligible voters</TableCell>
                        <TableCell align="right">Total votes counted</TableCell>
                        <TableCell align="right">Valid votes</TableCell>
                        <TableCell align="right">Participation</TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {resultsElections.map((result) => {
                        const election = elections.find((row) => row.id === result.election_id)
                        return (
                            <TableRow key={`${result.election_id}-${result.id}`}>
                                <TableCell>{result.name ?? translatedLabel(election, locale)}</TableCell>
                                <TableCell align="right">
                                    {valueOrDash(result.elegible_census)}
                                </TableCell>
                                <TableCell align="right">
                                    {valueOrDash(result.total_votes ?? result.total_valid_votes)}
                                </TableCell>
                                <TableCell align="right">
                                    {valueOrDash(result.total_valid_votes)}
                                </TableCell>
                                <TableCell align="right">
                                    {percent(
                                        result.total_votes_percent ??
                                            result.total_valid_votes_percent ??
                                            null
                                    )}
                                </TableCell>
                            </TableRow>
                        )
                    })}
                </TableBody>
            </Table>
        </TableContainer>
    </Box>
)
