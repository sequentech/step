// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Table, TableBody, TableCell, TableHead, TableRow, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {EColumnKind, POSITION_COLUMN, type MonitoringTable} from "./types"
import {formatCell} from "./lib/format"

export interface MonitoringDataTableProps {
    table: MonitoringTable
    /** Names the table for assistive technology. */
    caption: string
    /** A column's header; its raw name when not given. */
    columnLabel?: (name: string) => string
}

/** A widget's query result with exact values; the renderer's `position` column is left out. */
export function MonitoringDataTable({
    table,
    caption,
    columnLabel = (name) => name,
}: MonitoringDataTableProps) {
    const {t, i18n} = useTranslation()
    const shown = table.columns
        .map((column, index) => ({column, index}))
        .filter(({column}) => column.name !== POSITION_COLUMN)
    if (!table.rows.length) {
        return <Typography color="text.secondary">{t("monitoring.dataTable.empty")}</Typography>
    }
    return (
        <Box sx={{overflowX: "auto"}}>
            <Table size="small" aria-label={caption}>
                <TableHead>
                    <TableRow>
                        {shown.map(({column}) => (
                            <TableCell
                                key={column.name}
                                align={column.kind === EColumnKind.TEXT ? "left" : "right"}
                            >
                                {columnLabel(column.name)}
                            </TableCell>
                        ))}
                    </TableRow>
                </TableHead>
                <TableBody>
                    {table.rows.map((row, rowIndex) => (
                        <TableRow key={rowIndex}>
                            {shown.map(({column, index}) => (
                                <TableCell
                                    key={column.name}
                                    align={column.kind === EColumnKind.TEXT ? "left" : "right"}
                                    sx={{fontVariantNumeric: "tabular-nums"}}
                                >
                                    {formatCell(row[index], column.kind, i18n.language)}
                                </TableCell>
                            ))}
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </Box>
    )
}
