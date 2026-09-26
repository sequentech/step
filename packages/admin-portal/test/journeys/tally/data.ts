// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {createHash} from "node:crypto"
import type {Page} from "@playwright/test"
import {TENANT_ID} from "../fixtures"
import type {AdminPortal} from "../fixtures"

export const FIXED_TIME = "2026-01-01T00:00:00Z"
export const EVENT_ID = "21000000-0000-4000-8000-000000000001"
export const ELECTION_ID = "31000000-0000-4000-8000-000000000001"
export const CONTEST_ID = "71000000-0000-4000-8000-000000000001"
export const AREA_ID = "81000000-0000-4000-8000-000000000001"
export const ADMIN_ID = "91000000-0000-4000-8000-000000000001"

export type Row = Record<string, unknown>

export const sha256 = (bytes: Uint8Array | string) =>
    createHash("sha256").update(bytes).digest("hex")

/** Rows selected by a react-admin `where: {id: {_eq}}` or `{id: {_in}}` read. */
export function byId(rows: Row[], where: unknown): Row[] {
    const id = (where as {id?: {_eq?: unknown; _in?: unknown[]}} | undefined)?.id
    if (id?._eq !== undefined) return rows.filter((row) => row.id === id._eq)
    if (id?._in) return rows.filter((row) => id._in?.includes(row.id))
    return rows
}

/** Hasura's `distinct_on`: the first row of each group, in the fixture's order. */
function distinct(rows: Row[], columns: unknown): Row[] {
    if (!Array.isArray(columns)) return rows
    const seen = new Set<string>()
    return rows.filter((row) => {
        const key = JSON.stringify(columns.map((column) => row[String(column)]))
        if (seen.has(key)) return false
        seen.add(key)
        return true
    })
}

/**
 * Answers the react-admin list, get-one and get-many reads of a Hasura table.
 * Every read gets the current rows, so tests mutate them to model a backend update.
 */
export function table(portal: AdminPortal, name: string, records: () => Row[]) {
    portal.graphql.on(name, ({variables}) => {
        const rows = distinct(byId(records(), variables.where), variables.distinct_on)
        return {
            data: {
                [name]: rows,
                [`${name}_by_pk`]:
                    variables.id == null
                        ? (rows[0] ?? null)
                        : (rows.find((row) => row.id === variables.id) ?? null),
                [`${name}_aggregate`]: {aggregate: {count: rows.length}},
            },
        }
    })
}

export function electionEvent(overrides: Row = {}): Row {
    return {
        id: EVENT_ID,
        tenant_id: TENANT_ID,
        name: "Harbour event",
        alias: "Harbour event",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        labels: {},
        annotations: {},
        is_archived: false,
        encryption_protocol: "RSA256",
        presentation: {
            i18n: {en: {name: "Harbour event"}},
            language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
        },
        status: {is_published: true, voting_status: "CLOSED"},
        ...overrides,
    }
}

/** The event record and navigation tree every election event page reads. */
export function registerEvent(portal: AdminPortal, overrides: Row = {}) {
    const event = electionEvent(overrides)
    table(portal, "sequent_backend_election_event", () => [event])
    portal.graphql.on("election_events_tree", () => ({data: {sequent_backend_election_event: []}}))
    portal.graphql.on("election_tree", () => ({data: {sequent_backend_election: []}}))
    portal.graphql.on("contest_tree", () => ({data: {sequent_backend_contest: []}}))
    return event
}

export function adminUser(id = ADMIN_ID, username = "harbour-admin"): Row {
    return {
        id,
        username,
        email: `${username}@example.org`,
        email_verified: true,
        enabled: true,
        first_name: "Harbour",
        last_name: "Admin",
        attributes: {},
        groups: [],
        area: null,
        votes_info: [],
    }
}

/** Keycloak user lookups by ID, used to show who created a record. */
export function registerUsers(portal: AdminPortal, users: Row[] = [adminUser()]) {
    portal.graphql.on("getUsers", ({variables}) => {
        const ids = variables.userIds as string[] | undefined
        const items = ids ? users.filter((user) => ids.includes(String(user.id))) : users
        return {data: {get_users: {items, total: {aggregate: {count: items.length}}}}}
    })
}

/**
 * Records unhandled promise rejections instead of letting them fail the fixture, so an
 * expected-failure test can pin a missing error handler and still flip once it is fixed.
 */
export async function recordRejections(page: Page) {
    await page.addInitScript(() => {
        const log: string[] = []
        Object.assign(window, {unhandledRejections: log})
        window.addEventListener("unhandledrejection", (event) => {
            log.push(String(event.reason?.message ?? event.reason))
            event.preventDefault()
        })
    })
    return () =>
        page.evaluate(
            () => (window as unknown as {unhandledRejections: string[]}).unhandledRejections
        )
}
