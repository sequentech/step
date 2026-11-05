// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {MonitoringRenderWidgetResponse, MonitoringTable, MonitoringWidget} from "../types"

/** One query's rows in View data, with the query's definition for its column names. */
export interface DataSection {
    /** The query's name in the widget, or `null` for an older backend's one table. */
    query: string | null
    table: MonitoringTable
    definition?: unknown
}

type Queries = Record<string, unknown> | undefined

/** The name a widget's one `query` goes by: `sequent_core`'s `DEFAULT_QUERY_NAME`. */
export const DEFAULT_QUERY_NAME = "data"

/** A widget's queries by name; its one `query` is called `data`. */
export function widgetQueries(
    widget: Pick<MonitoringWidget, "query" | "queries"> | null | undefined
): Queries {
    if (widget?.queries && Object.keys(widget.queries).length) return widget.queries
    return widget?.query ? {[DEFAULT_QUERY_NAME]: widget.query} : undefined
}

/**
 * A section per governed query, in widget order. An older backend sends only
 * `table`, the first query's rows: one section, untitled.
 */
export function dataSections(
    render: Pick<MonitoringRenderWidgetResponse, "table" | "tables">,
    queries: Queries
): DataSection[] {
    if (render.tables?.length) {
        return render.tables.flatMap(({query, table}) =>
            table ? [{query, table, definition: queries?.[query]}] : []
        )
    }
    if (!render.table) return []
    const first = queries ? Object.values(queries)[0] : undefined
    return [{query: null, table: render.table, definition: first}]
}

/** A translation, if there is one for the key. */
export type LabelLookup = (key: string) => string | undefined

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

/** A ratio written out, `[voted, registered]`; not one a selector picks. */
function ratioMeasures(definition: Record<string, unknown>): [string, string] | undefined {
    const ratio = definition.ratio
    return Array.isArray(ratio) &&
        ratio.length === 2 &&
        ratio.every((measure) => typeof measure === "string")
        ? [ratio[0], ratio[1]]
        : undefined
}

/**
 * Column headers for a query's rows: the query's own `labels`, then the
 * platform's words for measures and result columns, then the raw name. A
 * ratio's numerator and denominator are named by their measures, and the
 * group column by the dimension grouped by.
 */
export function columnLabeler(definition: unknown, known: LabelLookup): (column: string) => string {
    const query = isRecord(definition) ? definition : {}
    const labels = isRecord(query.labels) ? query.labels : {}
    const ratio = ratioMeasures(query)
    const measure = (name: string): string | undefined => {
        const own = labels[name]
        return typeof own === "string" ? own : known(`monitoring.measures.${name}`)
    }
    return (column) => {
        if (ratio && column === "numerator") return measure(ratio[0]) ?? ratio[0]
        if (ratio && column === "denominator") return measure(ratio[1]) ?? ratio[1]
        if (column === "group" && typeof query.group_by === "string") {
            return known(`monitoring.columns.${query.group_by}`) ?? query.group_by
        }
        return measure(column) ?? known(`monitoring.columns.${column}`) ?? column
    }
}
