// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Chip, Typography} from "@mui/material"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {SyncDiffRow} from "./types"
import {CATEGORY_COLORS, CATEGORY_LABELS} from "./constants"

interface SyncDiffTableProps {
    rows: SyncDiffRow[]
    emptyMessage?: string
}

/**
 * Same visual language as ReviewChangesTable (old value struck through, new
 * value plain) but built on DataGrid since a reconciliation diff spans many
 * voters instead of a single record's fields. Callers split rows by `target`
 * into separate Datafix-side/Sequent-side tables, so this component doesn't
 * need to render which side a row belongs to. Client-side paginated: the
 * whole diff is fetched at once.
 */
export const SyncDiffTable: React.FC<SyncDiffTableProps> = ({
    rows,
    emptyMessage = "No differences found - the systems are in sync.",
}) => {
    const columns: GridColDef<SyncDiffRow>[] = [
        {field: "voterId", headerName: "Voter ID", width: 120},
        {field: "label", headerName: "Field", width: 180},
        {
            field: "category",
            headerName: "Category",
            width: 200,
            renderCell: (params: GridRenderCellParams<SyncDiffRow>) => (
                <Chip
                    size="small"
                    label={CATEGORY_LABELS[params.row.category]}
                    color={CATEGORY_COLORS[params.row.category]}
                />
            ),
        },
        {
            field: "oldValue",
            headerName: "Current value",
            width: 160,
            renderCell: (params: GridRenderCellParams<SyncDiffRow>) => (
                <Box component="del" sx={{opacity: 0.6}}>
                    {params.row.oldValue}
                </Box>
            ),
        },
        {field: "newValue", headerName: "New value", width: 160},
        {field: "failureReason", headerName: "Reason", flex: 1, minWidth: 220},
    ]

    if (rows.length === 0) {
        return (
            <Typography color="text.secondary" sx={{py: 2}}>
                {emptyMessage}
            </Typography>
        )
    }

    return (
        <DataGrid
            autoHeight
            rows={rows}
            columns={columns}
            density="compact"
            getRowId={(row) => row.id}
            initialState={{
                pagination: {
                    paginationModel: {pageSize: 10},
                },
            }}
            pageSizeOptions={[10, 25, 50]}
            disableRowSelectionOnClick
        />
    )
}

export default SyncDiffTable
