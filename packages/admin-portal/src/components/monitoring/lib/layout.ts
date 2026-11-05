// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {GRID_COLUMNS, type MonitoringDashboard, type MonitoringWidget} from "../types"

export interface ParsedWidgetEntry {
    widget: MonitoringWidget | null
    revision: number
    /** Why the definition cannot be drawn. */
    problem?: string
}

export interface LayoutCell {
    /** Unique within the dashboard: a widget may be placed more than once. */
    key: string
    widgetId: string
    width: number
    values: Record<string, string>
    widget: MonitoringWidget | null
    revision?: number
    problem?: string
}

/** A width the grid can draw: a whole number of columns from 1 to 12. */
export function clampWidth(width: number): number {
    if (!Number.isFinite(width)) return GRID_COLUMNS
    return Math.min(GRID_COLUMNS, Math.max(1, Math.round(width)))
}

/** Full width on small screens; the configured columns from md up. */
export function gridItemSize(width: number): {xs: number; md: number} {
    return {xs: GRID_COLUMNS, md: clampWidth(width)}
}

export function layoutCells(
    dashboard: MonitoringDashboard,
    widgets: Record<string, ParsedWidgetEntry>
): LayoutCell[] {
    return dashboard.layout.map((item, index) => {
        const entry = widgets[item.widget]
        return {
            key: `${index}:${item.widget}`,
            widgetId: item.widget,
            width: clampWidth(item.width),
            values: item.values ?? {},
            widget: entry?.widget ?? null,
            revision: entry?.revision,
            problem: entry ? entry.problem : "missing",
        }
    })
}
