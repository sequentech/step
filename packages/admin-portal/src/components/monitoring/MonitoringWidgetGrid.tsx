// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Grid} from "@mui/material"
import {GRID_COLUMNS} from "./types"
import {gridItemSize, type LayoutCell} from "./lib/layout"
import {MonitoringWidgetCard, type MonitoringWidgetContext} from "./MonitoringWidgetCard"

export interface MonitoringWidgetGridProps {
    cells: LayoutCell[]
    context: MonitoringWidgetContext
}

/** The layout on a 12-column grid; every widget takes the full row on small screens. */
export function MonitoringWidgetGrid({cells, context}: MonitoringWidgetGridProps) {
    return (
        <Grid container spacing={2} columns={GRID_COLUMNS}>
            {cells.map((cell) => (
                <Grid key={cell.key} size={gridItemSize(cell.width)}>
                    <MonitoringWidgetCard cell={cell} context={context} />
                </Grid>
            ))}
        </Grid>
    )
}
