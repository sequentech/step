// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Requests and results of the electoral-log console, which Harvest serves through
// the electoral_log_console_* actions.

export type ConsoleTable = "records" | "ballots" | "voters" | "queue"
export const CONSOLE_TABLES: ConsoleTable[] = ["records", "ballots", "voters", "queue"]

export type PageOrder = "newest-first" | "oldest-first"
export const PAGE_ORDERS: PageOrder[] = ["newest-first", "oldest-first"]

export type PersonalData = "shown" | "hidden"

export type FilterField =
    | "statement_kind"
    | "election_id"
    | "area_id"
    | "user_id"
    | "ballot_id"
    | "status"
    | "created_after"
    | "created_before"

// The filters each table applies, as Harvest does: the others are ignored.
export const FILTERS_BY_TABLE: Record<ConsoleTable, FilterField[]> = {
    records: [
        "statement_kind",
        "election_id",
        "user_id",
        "ballot_id",
        "created_after",
        "created_before",
    ],
    ballots: [
        "election_id",
        "area_id",
        "user_id",
        "ballot_id",
        "status",
        "created_after",
        "created_before",
    ],
    voters: ["election_id", "area_id", "user_id"],
    queue: ["election_id", "area_id"],
}

export const BALLOT_STATUSES = ["valid", "pending", "rejected"]

const DATE_FIELDS: FilterField[] = ["created_after", "created_before"]
// The ballot box keys elections and areas by UUID; records keep them as text.
const UUID_FIELDS: FilterField[] = ["election_id", "area_id"]
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

export type FilterValues = Partial<Record<FilterField, string>>

export interface ConsoleFilters {
    statement_kind?: string
    election_id?: string
    area_id?: string
    user_id?: string
    ballot_id?: string
    status?: string
    // Seconds since the epoch.
    created_after?: number
    created_before?: number
}

const filled = (values: FilterValues, field: FilterField): string | undefined => {
    const value = values[field]?.trim()
    return value ? value : undefined
}

/** The table's filters whose values the server would refuse. */
export const invalidFilters = (table: ConsoleTable, values: FilterValues): FilterField[] =>
    FILTERS_BY_TABLE[table].filter((field) => {
        const value = filled(values, field)
        if (value === undefined) {
            return false
        }
        if (DATE_FIELDS.includes(field)) {
            return Number.isNaN(Date.parse(value))
        }
        return table !== "records" && UUID_FIELDS.includes(field) && !UUID_PATTERN.test(value)
    })

/** The filters of a page request: the table's filled values, dates in seconds. */
export const toConsoleFilters = (table: ConsoleTable, values: FilterValues): ConsoleFilters => {
    const filters: ConsoleFilters = {}
    for (const field of FILTERS_BY_TABLE[table]) {
        const value = filled(values, field)
        if (value === undefined) {
            continue
        }
        if (field === "created_after" || field === "created_before") {
            filters[field] = Math.floor(Date.parse(value) / 1000)
        } else {
            filters[field] = value
        }
    }
    return filters
}

export interface ConsoleRows {
    columns: string[]
    rows: unknown[][]
}

export interface ConsolePage extends ConsoleRows {
    // Cursor of the next page, or null at the end.
    next: string | null
    estimated_rows: number
    personal_columns: string[]
    personal_data: PersonalData
}

export interface ConsoleQueryRows extends ConsoleRows {
    truncated: boolean
    elapsed_ms: number
}

export interface ConsoleQueryError {
    error: string
}

export type ConsoleQueryResult = ConsoleQueryRows | ConsoleQueryError

export const isQueryError = (result: ConsoleQueryResult): result is ConsoleQueryError =>
    typeof (result as ConsoleQueryError).error === "string"

// What the super-admin tenant may browse: every tenant, its election events and
// their elections, as electoral_log_console_tenants returns them.
export interface ConsoleElection {
    id: string
    presentation?: unknown
}

export interface ConsoleEvent extends ConsoleElection {
    is_archived: boolean
    elections: ConsoleElection[]
}

export interface ConsoleTenant {
    id: string
    slug: string
    events: ConsoleEvent[]
}

export interface ConsoleTenants {
    tenants: ConsoleTenant[]
}

/** The election events of a tenant, or none if the tenant is unknown. */
export const tenantEvents = (tenants: ConsoleTenant[], tenantId: string): ConsoleEvent[] =>
    tenants.find((tenant) => tenant.id === tenantId)?.events ?? []

/** The elections of an election event of a tenant, or none if either is unknown. */
export const eventElections = (
    tenants: ConsoleTenant[],
    tenantId: string,
    eventId: string
): ConsoleElection[] =>
    tenantEvents(tenants, tenantId).find((event) => event.id === eventId)?.elections ?? []

export const ROW_ID = "__row"

/** The grid field of a column: by position, since a query may repeat a name. */
export const columnField = (index: number): string => `c${index}`

export type GridRow = Record<string, unknown>

/** Rows as objects for a data grid, numbered from `offset`. */
export const toGridRows = ({columns, rows}: ConsoleRows, offset = 0): GridRow[] =>
    rows.map((row, index) => {
        const object: GridRow = {[ROW_ID]: offset + index}
        columns.forEach((_, column) => {
            object[columnField(column)] = row[column] ?? null
        })
        return object
    })

/** How a value shows in a cell. */
export const displayValue = (value: unknown): string => {
    if (value === null || value === undefined) {
        return ""
    }
    if (typeof value === "object") {
        return JSON.stringify(value)
    }
    return String(value)
}
