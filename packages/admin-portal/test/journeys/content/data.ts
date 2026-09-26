// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Locator, Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import type {GraphQLCall, GraphQLReply} from "@sequentech/ui-test-kit/mocks/graphql"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {Kind, parse, print, visit} from "graphql"
import {expect, TENANT_ID} from "../fixtures"

export type Row = Record<string, unknown>

export const CONTENT_IDS = {
    event: IDS.event,
    election: IDS.election,
    secondElection: "30000000-0000-4000-8000-000000000002",
    area: IDS.area,
    secondArea: "40000000-0000-4000-8000-000000000002",
    contest: IDS.contest,
    secondContest: "60000000-0000-4000-8000-000000000002",
    candidate: "90000000-0000-4000-8000-000000000001",
    secondCandidate: "90000000-0000-4000-8000-000000000002",
    document: "a0000000-0000-4000-8000-000000000001",
    template: "b0000000-0000-4000-8000-000000000001",
    report: "c0000000-0000-4000-8000-000000000001",
    task: "d0000000-0000-4000-8000-000000000001",
    scheduledEvent: "e0000000-0000-4000-8000-000000000001",
    supportMaterial: "f0000000-0000-4000-8000-000000000001",
} as const

export const EVENT_SCOPE = {tenant_id: TENANT_ID, election_event_id: IDS.event}

/** Roles every content journey needs to open the portal and an event. */
export const BASE_ROLES = ["admin-user", "election-event-read", "election-read"]

export const names = (name: string, extra: Row = {}) => ({i18n: {en: {name, ...extra}}})

/** New ballot items copy their texts into every admin portal language. */
export const everyLanguage = (texts: Row) =>
    Object.fromEntries(
        ["cat", "en", "es", "eu", "fr", "gl", "nl", "tl"].map((lang) => [lang, texts])
    )

export function eventRow(overrides: Row = {}): Row {
    return {
        id: IDS.event,
        tenant_id: TENANT_ID,
        name: "Council",
        alias: null,
        description: "Council election",
        encryption_protocol: "RSA256",
        is_archived: false,
        is_audit: false,
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        presentation: {
            ...names("Council"),
            language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
        },
        status: {},
        voting_channels: {online: true},
        ...overrides,
    }
}

export function electionRow(overrides: Row = {}): Row {
    return {
        id: IDS.election,
        ...EVENT_SCOPE,
        name: "Mayor election",
        alias: null,
        description: "Election of the mayor",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        presentation: names("Mayor election"),
        status: {},
        voting_channels: {online: true},
        num_allowed_revotes: 1,
        permission_label: null,
        ...overrides,
    }
}

export function contestRow(overrides: Row = {}): Row {
    return {
        id: IDS.contest,
        ...EVENT_SCOPE,
        election_id: IDS.election,
        name: "Mayor",
        alias: null,
        description: "Choose the mayor",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        presentation: names("Mayor"),
        min_votes: 0,
        max_votes: 1,
        winning_candidates_num: 1,
        is_acclaimed: false,
        is_active: true,
        is_encrypted: true,
        voting_type: "no-preferential",
        counting_algorithm: "plurality-at-large",
        ...overrides,
    }
}

export function candidateRow(overrides: Row = {}): Row {
    return {
        id: CONTENT_IDS.candidate,
        ...EVENT_SCOPE,
        contest_id: IDS.contest,
        name: "Alice Adams",
        alias: null,
        description: "Independent",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        presentation: names("Alice Adams"),
        is_public: true,
        type: null,
        ...overrides,
    }
}

export function areaRow(overrides: Row = {}): Row {
    return {
        id: IDS.area,
        ...EVENT_SCOPE,
        name: "North",
        description: "Northern district",
        type: null,
        parent_id: null,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        presentation: {},
        ...overrides,
    }
}

type Operators = Record<string, unknown>
const isRecord = (value: unknown): value is Row =>
    typeof value === "object" && value !== null && !Array.isArray(value)
const likePattern = (pattern: string) =>
    new RegExp(
        `^${pattern
            .split("%")
            .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
            .join(".*")}$`,
        "i"
    )
function contains(value: unknown, expected: unknown): boolean {
    if (Array.isArray(expected))
        return Array.isArray(value) && expected.every((item) => value.includes(item))
    if (isRecord(expected))
        return isRecord(value) && Object.entries(expected).every(([k, v]) => contains(value[k], v))
    return value === expected
}
function compare(value: unknown, operators: Operators): boolean {
    return Object.entries(operators).every(([operator, expected]) => {
        switch (operator) {
            case "_eq":
                return value === expected
            case "_neq":
                return value !== expected
            case "_in":
                return (expected as unknown[]).includes(value)
            case "_nin":
                return !(expected as unknown[]).includes(value)
            case "_like":
            case "_ilike":
                return typeof value === "string" && likePattern(String(expected)).test(value)
            case "_is_null":
                return (value == null) === expected
            case "_contains":
                return contains(value, expected)
            case "_gt":
                return String(value) > String(expected)
            case "_gte":
                return String(value) >= String(expected)
            case "_lt":
                return String(value) < String(expected)
            case "_lte":
                return String(value) <= String(expected)
            default:
                throw new Error(`Unsupported Hasura operator ${operator}`)
        }
    })
}

/** Evaluates the subset of Hasura boolean expressions that react-admin and the portal send. */
export function matches(row: Row, where: unknown): boolean {
    if (!isRecord(where)) return true
    return Object.entries(where).every(([key, condition]) => {
        if (key === "_and")
            return (Array.isArray(condition) ? condition : [condition]).every((part) =>
                matches(row, part)
            )
        if (key === "_or") return (condition as unknown[]).some((part) => matches(row, part))
        if (key === "_not") return !matches(row, condition)
        if (!isRecord(condition)) return false
        const value = row[key]
        return Object.keys(condition).every((name) => name.startsWith("_"))
            ? compare(value, condition)
            : isRecord(value) && matches(value, condition)
    })
}

export const listReply = (resource: string, rows: Row[]): GraphQLReply => ({
    data: {[resource]: rows, [`${resource}_aggregate`]: {aggregate: {count: rows.length}}},
})

/**
 * An in-memory Hasura table behind react-admin's generated operations: list/one
 * (`<resource>`), `insert_`, `update_` and `delete_`. Tests read `rows` to see
 * the stored state and `portal.graphql.callsTo` for the exact wire variables.
 */
export function table(
    portal: PortalServices,
    resource: string,
    initial: Row[] = [],
    newId: () => string = () => {
        throw new Error(`No id generator for ${resource}`)
    }
) {
    const rows = initial.map((row) => ({...row}))
    portal.graphql.on(resource, ({variables}: GraphQLCall) => {
        const selected = rows.filter((row) => matches(row, variables.where))
        const offset = Number(variables.offset ?? 0)
        const limit = variables.limit == null ? selected.length : Number(variables.limit)
        return {
            data: {
                [resource]: selected.slice(offset, offset + limit),
                [`${resource}_aggregate`]: {aggregate: {count: selected.length}},
            },
        }
    })
    portal.graphql.on(`insert_${resource}`, ({variables}) => {
        const objects = [variables.objects].flat() as Row[]
        const inserted = objects.map((object) => ({
            id: newId(),
            created_at: FIXED_TIME,
            last_updated_at: FIXED_TIME,
            ...object,
        }))
        rows.push(...inserted)
        return {
            data: {
                [`insert_${resource}`]: {returning: inserted, affected_rows: inserted.length},
            },
        }
    })
    portal.graphql.on(`update_${resource}`, ({variables}) => {
        const updated = rows.filter((row) => matches(row, variables.where))
        for (const row of updated) Object.assign(row, variables._set)
        return {
            data: {[`update_${resource}`]: {returning: updated, affected_rows: updated.length}},
        }
    })
    portal.graphql.on(`delete_${resource}`, ({variables}) => {
        const removed = rows.filter((row) => matches(row, variables.where))
        for (const row of removed) rows.splice(rows.indexOf(row), 1)
        return {
            data: {[`delete_${resource}`]: {returning: removed, affected_rows: removed.length}},
        }
    })
    return rows
}

/** Registers the event and its (empty) tree so `/sequent_backend_election_event/:id` renders. */
export function eventPage(portal: PortalServices, event: Row = eventRow(), elections: Row[] = []) {
    table(portal, "sequent_backend_election_event", [event])
    portal.graphql.on("election_events_tree", () => ({
        data: {sequent_backend_election_event: [{...event, elections}]},
    }))
    portal.graphql.on("election_tree", () => ({data: {sequent_backend_election: elections}}))
    return event
}

export async function openEventTab(page: Page, portal: PortalServices, tab: string) {
    await page.goto(`${portal.origin}/sequent_backend_election_event/${IDS.event}?lang=en`)
    const tabButton = page.getByRole("tab", {name: tab, exact: true})
    await tabButton.click()
    await expect(tabButton).toHaveAttribute("aria-selected", "true")
}

/** Every recorded call to `operation` was sent with the given Hasura role. */
export function expectRole(portal: PortalServices, operation: string, role: string) {
    const calls = portal.graphql.callsTo(operation)
    expect(calls.length, `${operation} was requested`).toBeGreaterThan(0)
    expect(
        calls.map((call) => call.headers["x-hasura-role"]),
        `${operation} x-hasura-role`
    ).toEqual(calls.map(() => role))
}

/**
 * A row action rendered as an icon button without an accessible name, found by
 * the icon's e2e class hook (production MUI icons carry no test id).
 */
export const iconButton = (scope: Locator, iconClass: string) =>
    scope.getByRole("button").filter({has: scope.page().locator(`.${iconClass}`)})

/** Snackbars stay outside an open drawer, which MUI hides from the accessibility tree. */
export const notification = (page: Page, text: string) =>
    page.getByRole("alert", {includeHidden: true}).filter({hasText: text})

/**
 * React-admin shows one notification at a time and MUI pauses its auto-hide
 * while hovered: move the pointer away and let the frozen clock expire it.
 */
export async function expireNotification(page: Page) {
    await page.mouse.move(0, 0)
    await page.clock.runFor(5000)
}

/**
 * Answers a known-invalid operation the way Hasura does, so a journey can go on
 * past a pinned product defect. Returns the intercepted calls.
 */
export function answerInvalid(
    portal: PortalServices,
    operationName: string,
    isInvalid: (variables: Row) => boolean,
    message: string
) {
    const intercepted: Row[] = []
    const handle = portal.graphql.handle.bind(portal.graphql)
    portal.graphql.handle = async (request) => {
        const body = request.body ? (JSON.parse(request.body) as Row) : {}
        const variables = (body.variables ?? {}) as Row
        if (body.operationName === operationName && isInvalid(variables)) {
            intercepted.push(variables)
            return {
                status: 200,
                headers: {"content-type": "application/json"},
                body: JSON.stringify({
                    errors: [{message, extensions: {code: "validation-failed", path: "$"}}],
                }),
            }
        }
        return handle(request)
    }
    return intercepted
}

/**
 * Lets a journey go on past an operation that declares variables it never uses:
 * GraphQL validation rejects the document, Hasura tolerates it. Only the named
 * definitions are dropped before the strict mock validates; the original
 * documents are returned so a pinned test can check them.
 */
export function dropUnusedVariables(
    portal: PortalServices,
    operationName: string,
    unused: string[]
) {
    const originals: string[] = []
    const handle = portal.graphql.handle.bind(portal.graphql)
    portal.graphql.handle = async (request) => {
        const body = request.body ? (JSON.parse(request.body) as Row) : {}
        if (body.operationName !== operationName || typeof body.query !== "string")
            return handle(request)
        originals.push(body.query)
        const query = print(
            visit(parse(body.query), {
                [Kind.VARIABLE_DEFINITION]: (node) =>
                    unused.includes(node.variable.name.value) ? null : undefined,
            })
        )
        return handle({...request, body: JSON.stringify({...body, query})})
    }
    return originals
}

/**
 * Handles the page's unhandled rejections whose message contains `message`, so a
 * pinned defect does not reach the page-error log. Returns the handled messages.
 */
export async function catchRejections(page: Page, message: string) {
    await page.addInitScript((expected) => {
        const pinned = window as unknown as {pinnedRejections: string[]}
        pinned.pinnedRejections = []
        window.addEventListener("unhandledrejection", (event) => {
            const reason = String(event.reason?.message ?? event.reason)
            if (!reason.includes(expected)) return
            event.preventDefault()
            pinned.pinnedRejections.push(reason)
        })
    }, message)
    return () =>
        page.evaluate(() => (window as unknown as {pinnedRejections: string[]}).pinnedRejections)
}

/**
 * The checked-in client schema predates some Hasura columns (for example
 * `report.permission_label`, added by migration 1748425581311), so the strict
 * mock would reject writes that carry them. Drops only the named input fields
 * before validation and returns the original variables for assertions.
 */
export function allowUnlistedInputFields(
    portal: PortalServices,
    operationName: string,
    variable: string,
    fields: string[]
) {
    const originals: Row[] = []
    const handle = portal.graphql.handle.bind(portal.graphql)
    portal.graphql.handle = async (request) => {
        const body = request.body ? (JSON.parse(request.body) as Row) : {}
        if (body.operationName !== operationName) return handle(request)
        const variables = {...(body.variables as Row)}
        originals.push(structuredClone(variables))
        const input = {...(variables[variable] as Row)}
        for (const field of fields) delete input[field]
        variables[variable] = input
        return handle({...request, body: JSON.stringify({...body, variables})})
    }
    return originals
}
