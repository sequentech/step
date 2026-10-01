// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
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
import type {IMonitoringTable} from "./types"

export const QUERY_RESULT_ROWS = 5

export interface MonitoringQueryResultProps {
    table?: IMonitoringTable | null
    rows?: number
}

const cell = (value: unknown) =>
    value === null || value === undefined
        ? "—"
        : typeof value === "object"
          ? JSON.stringify(value)
          : String(value)

/** Query result · first rows: what the query returned for the preview, exactly. */
export const MonitoringQueryResult: React.FC<MonitoringQueryResultProps> = ({
    table,
    rows = QUERY_RESULT_ROWS,
}) => {
    const {t} = useTranslation()
    const title = t("monitoring.editor.preview.queryResult")
    return (
        <Box component="section" aria-label={title}>
            <Typography variant="subtitle2" component="h3" sx={{mb: 1}}>
                {title}
            </Typography>
            {!table || table.rows.length === 0 ? (
                <Typography variant="body2" color="text.secondary">
                    {t("monitoring.editor.preview.noRows")}
                </Typography>
            ) : (
                <TableContainer sx={{overflowX: "auto"}}>
                    <Table size="small" aria-label={title}>
                        <TableHead>
                            <TableRow>
                                {table.columns.map((column) => (
                                    <TableCell key={column.name}>{column.name}</TableCell>
                                ))}
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            {table.rows.slice(0, rows).map((row, index) => (
                                <TableRow key={index}>
                                    {table.columns.map((column, position) => (
                                        <TableCell key={column.name}>
                                            {cell(row[position])}
                                        </TableCell>
                                    ))}
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </TableContainer>
            )}
        </Box>
    )
}

export default MonitoringQueryResult
