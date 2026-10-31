// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {
    Box,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TablePagination,
    TableRow,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {EColumnKind, POSITION_COLUMN, type MonitoringTable} from "./types"
import {formatCell, formatInteger} from "./lib/format"

/** Rows shown at once; a table of this many or fewer has no pager. */
export const DATA_TABLE_PAGE_SIZE = 100

/** Page sizes the viewer can pick; the smallest is the default. */
export const DATA_TABLE_PAGE_SIZES = [DATA_TABLE_PAGE_SIZE, 250, 1000]

export interface MonitoringDataTableProps {
    table: MonitoringTable
    /** Names the table for assistive technology. */
    caption: string
    /** A column's header; its raw name when not given. */
    columnLabel?: (name: string) => string
}

/**
 * A widget's query result with exact values; the renderer's `position` column
 * is left out. A table longer than a page is shown a page at a time, so that
 * thousands of rows do not all become part of the document at once.
 */
export function MonitoringDataTable({
    table,
    caption,
    columnLabel = (name) => name,
}: MonitoringDataTableProps) {
    const {t, i18n} = useTranslation()
    const [page, setPage] = useState(0)
    const [pageSize, setPageSize] = useState(DATA_TABLE_PAGE_SIZE)
    const shown = table.columns
        .map((column, index) => ({column, index}))
        .filter(({column}) => column.name !== POSITION_COLUMN)
    if (!table.rows.length) {
        return <Typography color="text.secondary">{t("monitoring.dataTable.empty")}</Typography>
    }
    const paged = table.rows.length > DATA_TABLE_PAGE_SIZE
    // A refresh can bring fewer rows: the last page there is, never an empty one.
    const lastPage = Math.max(0, Math.ceil(table.rows.length / pageSize) - 1)
    const current = paged ? Math.min(page, lastPage) : 0
    const rows = paged ? table.rows.slice(current * pageSize, (current + 1) * pageSize) : table.rows
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
                    {rows.map((row, rowIndex) => (
                        <TableRow key={current * pageSize + rowIndex}>
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
            {paged ? (
                <TablePagination
                    component="div"
                    count={table.rows.length}
                    page={current}
                    rowsPerPage={pageSize}
                    rowsPerPageOptions={DATA_TABLE_PAGE_SIZES}
                    onPageChange={(_event, next) => setPage(next)}
                    onRowsPerPageChange={(event) => {
                        setPageSize(Number(event.target.value))
                        setPage(0)
                    }}
                    showFirstButton
                    showLastButton
                    labelRowsPerPage={t("monitoring.dataTable.rowsPerPage")}
                    labelDisplayedRows={({from, to, count}) =>
                        t("monitoring.dataTable.shownRows", {
                            from: formatInteger(from, i18n.language),
                            to: formatInteger(to, i18n.language),
                            total: formatInteger(count, i18n.language),
                        })
                    }
                    getItemAriaLabel={(type) => t(`monitoring.dataTable.${type}Page`)}
                />
            ) : null}
        </Box>
    )
}
