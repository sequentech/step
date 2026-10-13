// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {expect, type Page} from "@playwright/test"
import type {GraphQLHandler} from "@sequentech/ui-test-kit/mocks/graphql"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {TENANT_ID, type AdminPortal} from "../fixtures"

type Row = Record<string, unknown>
/** The settings tabs but Logs require both roles; specs add the per-tab permissions they exercise. */
export const SETTINGS_ROLES = ["admin-user", "tenant-write", "settings-menu"]
/** A read-only auditor: no admin-user role, and only the Logs tab of Settings. */
export const AUDITOR_ROLES = [
    "election-event-read",
    "settings-menu",
    "tenant-logs-read",
    "tenant-logs-export",
]
export const TENANT_WHERE = {id: {_eq: TENANT_ID}}
export const LANGUAGE_CONF = {enabled_language_codes: ["en"], default_language_code: "en"}

/** A list, getOne or getMany reply for a react-admin (ra-data-hasura) resource. */
function listReply(resource: string, rows: Row[]) {
    return {data: {[resource]: rows, [`${resource}_aggregate`]: {aggregate: {count: rows.length}}}}
}

export function listOf(resource: string, rows: () => Row[]): GraphQLHandler {
    return () => listReply(resource, rows())
}

/** The signed-in tenant with every non-null column the portal selects. */
export function tenantRow(overrides: Row = {}): Row {
    return {
        id: TENANT_ID,
        slug: "synthetic",
        is_active: true,
        test: 1,
        labels: {},
        annotations: {},
        settings: {language_conf: LANGUAGE_CONF},
        voting_channels: {online: true, kiosk: false, telephone: false},
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        ...overrides,
    }
}

/**
 * Serves one tenant row to every tenant query and applies each accepted
 * `update_sequent_backend_tenant` `_set` to it, as Hasura would, so the portal
 * refetches what it saved.
 */
export function mockTenant(portal: AdminPortal, initial: Row = tenantRow()) {
    let current = initial
    portal.graphql.on(
        "sequent_backend_tenant",
        listOf("sequent_backend_tenant", () => [current])
    )
    portal.graphql.on("update_sequent_backend_tenant", ({variables}) => {
        current = {...current, ...(variables._set as Row)}
        return {data: {update_sequent_backend_tenant: {affected_rows: 1, returning: [current]}}}
    })
    return {
        current: () => current,
        updates: () =>
            portal.graphql.callsTo("update_sequent_backend_tenant").map((call) => call.variables),
    }
}

export const ELECTION_TYPE_ID = "20000000-0000-4000-8000-000000000001"
const NEW_ELECTION_TYPE_ID = "20000000-0000-4000-8000-000000000002"
export function electionType(overrides: Row = {}): Row {
    return {
        id: ELECTION_TYPE_ID,
        tenant_id: TENANT_ID,
        name: "General election",
        annotations: {},
        labels: {},
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        ...overrides,
    }
}

/**
 * A stateful react-admin table: list, getOne, insert, update and delete calls
 * read and change the same rows, so the portal refetches what it wrote.
 */
function mockTable(
    portal: AdminPortal,
    resource: string,
    initial: Row[],
    created: (objects: Row) => Row
) {
    let rows = [...initial]
    const idOf = (where: unknown) => (where as {id?: {_eq?: string}} | undefined)?.id?._eq
    const matching = (where: unknown) => rows.filter((row) => row.id === idOf(where))
    portal.graphql.on(resource, ({variables}) =>
        listReply(resource, idOf(variables.where) ? matching(variables.where) : rows)
    )
    portal.graphql.on(`insert_${resource}`, ({variables}) => {
        const row = created(variables.objects as Row)
        rows = [...rows, row]
        return {data: {[`insert_${resource}`]: {affected_rows: 1, returning: [row]}}}
    })
    portal.graphql.on(`update_${resource}`, ({variables}) => {
        const id = idOf(variables.where)
        rows = rows.map((row) => (row.id === id ? {...row, ...(variables._set as Row)} : row))
        return {
            data: {
                [`update_${resource}`]: {affected_rows: 1, returning: matching(variables.where)},
            },
        }
    })
    portal.graphql.on(`delete_${resource}`, ({variables}) => {
        const deleted = matching(variables.where)
        rows = rows.filter((row) => !deleted.includes(row))
        return {data: {[`delete_${resource}`]: {affected_rows: 1, returning: deleted}}}
    })
    return {rows: () => rows}
}

export function mockElectionTypes(portal: AdminPortal, initial: Row[] = []) {
    return mockTable(portal, "sequent_backend_election_type", initial, (objects) =>
        electionType({...objects, id: NEW_ELECTION_TYPE_ID})
    )
}

export const TRUSTEE_ID = "30000000-0000-4000-8000-000000000001"
const NEW_TRUSTEE_ID = "30000000-0000-4000-8000-000000000002"
export function trustee(overrides: Row = {}): Row {
    return {
        id: TRUSTEE_ID,
        tenant_id: TENANT_ID,
        name: "Trustee Alice",
        public_key: "alice-public-key",
        annotations: {},
        labels: {},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        ...overrides,
    }
}

export function mockTrustees(portal: AdminPortal, initial: Row[] = []) {
    return mockTable(portal, "sequent_backend_trustee", initial, (objects) =>
        trustee({...objects, id: NEW_TRUSTEE_ID})
    )
}

/** A task row as the export and import actions return it. */
export function taskExecution(type: string, overrides: Row = {}): Row {
    return {
        id: "40000000-0000-4000-8000-000000000001",
        tenant_id: TENANT_ID,
        election_event_id: IDS.event,
        name: type,
        type,
        execution_status: "IN_PROGRESS",
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: null,
        executed_by_user: IDS.voter,
        annotations: {},
        labels: {},
        logs: [],
        ...overrides,
    }
}

/** Serves a generated document through GetDocument and FetchDocument's presigned S3 URL. */
export function serveDocument(portal: AdminPortal, documentId: string, name: string) {
    const url = portal.s3.presign(`${TENANT_ID}/documents/${documentId}/${name}`, "download")
    portal.graphql.on("GetDocument", () => ({
        data: {sequent_backend_document: [{id: documentId, name, annotations: {}}]},
    }))
    portal.graphql.on("FetchDocument", () => ({data: {fetchDocument: {url}}}))
    return {url}
}

/** Answers the task widget's polling with the given task row. */
export function mockTask(portal: AdminPortal, task: () => Row) {
    portal.graphql.on("GetTaskById", () => ({data: {sequent_backend_tasks_execution: [task()]}}))
}

/** The switch labelled `label`; the settings switches have no accessible names of their own. */
export function switchFor(page: Page, label: string) {
    return page
        .locator("div")
        .filter({hasText: new RegExp(`^${label}$`)})
        .filter({has: page.getByRole("switch")})
        .getByRole("switch")
}

/** The table row that has a cell showing exactly `text`. */
export function rowWith(page: Page, text: string) {
    return page.getByRole("row").filter({has: page.getByRole("cell", {name: text, exact: true})})
}

/** Opens the settings screen, optionally switching to the tab labelled `tab`. */
export async function openSettings(page: Page, portal: AdminPortal, tab?: string) {
    await page.goto(`${portal.origin}/settings?lang=en`)
    await expect(page.getByRole("tab", {name: "ELECTION TYPES", exact: true})).toBeVisible()
    if (tab) await page.getByRole("tab", {name: tab, exact: true}).click()
}

/**
 * Settings tabs save through react-admin's undoable mode: the mutation leaves
 * only once the "Undo" notification closes, so the fake clock is advanced
 * until the expected call count is reached.
 */
export async function commitUndoable(
    page: Page,
    portal: AdminPortal,
    operation: string,
    count = 1
) {
    await expect
        .poll(async () => {
            await page.clock.runFor(1_000)
            return portal.graphql.callsTo(operation).length
        })
        .toBe(count)
}

export const TENANT_LOG_SUBTITLE =
    "Sign-ins, staff accounts and roles, settings and election events of this organization. Each election event keeps its own log in its Logs tab."
export const SETTINGS_CHANGED = "Settings changed: Languages and Integrations"
export const SIGN_IN_FAILED = "Sign-in failed: wrong password"
export const KEYCLOAK_CHANGE = "Keycloak: user lena.ortiz created by Step's service account"
/** 3 February 2028, 09:21:37 UTC. */
const TENANT_LOG_TIME = Date.parse("2028-02-03T09:21:37Z") / 1000

interface ITenantLogEntry {
    id: number
    kind: string
    username: string | null
    eventType: "USER" | "SYSTEM"
    logType: "INFO" | "ERROR"
    description: string
    body: Row
}

const auditBody = (kind: string, details: Row): Row => ({
    Audit: {kind, details_json: JSON.stringify(details), entry_id: `entry-${kind}`},
})

const TENANT_LOG: ITenantLogEntry[] = [
    {
        id: 5259,
        kind: "AdminChange",
        username: "rosa.marin",
        eventType: "USER",
        logType: "INFO",
        description: SETTINGS_CHANGED,
        body: auditBody("AdminChange", {
            object: {type: "tenant", id: TENANT_ID, name: "Synthetic"},
            action: "update",
            actor: {user_id: IDS.voter, username: "rosa.marin", role: "admin-user"},
            origin: "hasura",
            changes: [
                {path: "settings.logs_timezone", old: "UTC", new: "Europe/Madrid"},
                {path: "settings.gapi_key", secret: true},
            ],
        }),
    },
    {
        id: 5258,
        kind: "KeycloakAdminEvent",
        username: null,
        eventType: "SYSTEM",
        logType: "INFO",
        description: KEYCLOAK_CHANGE,
        body: auditBody("KeycloakAdminEvent", {
            operation: "CREATE",
            resource_type: "USER",
            resource_path: "users/60000000-0000-4000-8000-000000000001",
            representation: {username: "lena.ortiz", enabled: true},
            error: null,
            actor: {username: "service-account", client_id: "service-account", realm: "tenant"},
            made_by: "SERVICE_ACCOUNT",
        }),
    },
    {
        id: 5257,
        kind: "KeycloakUserEvent",
        username: "lena.ortiz",
        eventType: "USER",
        logType: "ERROR",
        description: SIGN_IN_FAILED,
        body: {
            KeycloakUserEvent: [
                JSON.stringify({
                    error: "invalid_user_credentials",
                    client_id: "admin-portal",
                    ip_address: "203.0.113.77",
                    auth_method: "openid-connect",
                    auth_type: "code",
                    credential_type: "password",
                }),
                "LOGIN_ERROR",
            ],
        },
    },
]

/**
 * Answers the tenant log's list (`listTenantLog`) with three entries, newest
 * first, keeping only those the request's filter asks for.
 */
export function mockTenantLog(portal: AdminPortal) {
    portal.graphql.on("listTenantLog", ({variables}) => {
        const filter = (variables.filter ?? {}) as Record<string, string | undefined>
        const items = TENANT_LOG.filter(
            (entry) =>
                (!filter.event_type || filter.event_type === entry.eventType) &&
                (!filter.log_type || filter.log_type === entry.logType) &&
                (!filter.statement_kind || filter.statement_kind === entry.kind) &&
                (!filter.username || filter.username === entry.username)
        ).map((entry, index) => ({
            id: entry.id,
            created: TENANT_LOG_TIME - index * 60 + 1,
            statement_timestamp: TENANT_LOG_TIME - index * 60,
            statement_kind: entry.kind,
            // The client schema makes the row's user_id non-null, though SYSTEM entries have none.
            user_id: entry.username ? IDS.voter : "",
            message: JSON.stringify({
                sender: {name: entry.eventType === "USER" ? "Rosa Marin" : "", pk: "admin-key"},
                sender_signature: "sender-signature",
                system_signature: "system-signature",
                user_id: entry.username ? IDS.voter : null,
                username: entry.username,
                statement: {
                    head: {
                        kind: entry.kind,
                        event_type: entry.eventType,
                        log_type: entry.logType,
                        description: entry.description,
                    },
                    body: entry.body,
                },
            }),
        }))
        return {data: {listElectoralLog: {items, total: {aggregate: {count: items.length}}}}}
    })
}
