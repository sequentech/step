// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {createHash} from "node:crypto"
import {createRequire} from "node:module"
import initSqlJs from "sql.js"
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

export const KEYS_ID = "41000000-0000-4000-8000-000000000001"
export const TALLY_ID = "61000000-0000-4000-8000-000000000001"
export const EXECUTION_ID = "61000000-0000-4000-8000-000000000002"
export const RESULTS_ID = "61000000-0000-4000-8000-000000000003"
export const ALICE_ID = "c1000000-0000-4000-8000-000000000001"
export const BOB_ID = "c1000000-0000-4000-8000-000000000002"
export const TRUSTEES = ["Alice Trustee", "Bob Trustee"]

export const TALLY_ROLES = [
    "admin-user",
    "election-event-read",
    "election-read",
    "admin-ceremony",
    "keys-read",
    "tally-read",
    "tally-start",
    "tally-write",
    "tally-results-read",
    "tally-resolution-submit",
    "tally-recount-execute",
    "election-event-tally-tab",
    "election-event-tally-columns",
]

/**
 * A closed, published event with a finished manual keys ceremony, one election with one
 * contest (Alice and Bob), and a tally session whose state the test drives.
 */
export function tallyWorld(portal: AdminPortal) {
    const event = registerEvent(portal, {
        presentation: {
            i18n: {en: {name: "Harbour event"}},
            language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
            ceremonies_policy: "manual-ceremonies",
        },
    })
    const trustees = TRUSTEES.map((name, index) => ({
        id: `51000000-0000-4000-8000-00000000000${index + 1}`,
        name,
        tenant_id: TENANT_ID,
        public_key: `synthetic-public-key-${index + 1}`,
        labels: {},
        annotations: {},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }))
    portal.graphql.on("TrusteeNames", () => ({data: {sequent_backend_trustee: trustees}}))
    table(portal, "sequent_backend_trustee", () => trustees)
    const keys = {
        id: KEYS_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        trustee_ids: trustees.map((trustee) => trustee.id),
        status: {public_key: "synthetic-public-key", trustees: [], logs: []},
        execution_status: "SUCCESS",
        labels: {},
        annotations: {},
        threshold: 2,
        name: "Harbour key",
        settings: {policy: "manual-ceremonies"},
        is_default: true,
        permission_label: null,
    }
    portal.graphql.on("ListKeysCeremony", () => ({
        data: {list_keys_ceremony: {items: [keys], total: {aggregate: {count: 1}}}},
    }))
    table(portal, "sequent_backend_keys_ceremony", () => [keys])
    const scope = {tenant_id: TENANT_ID, election_event_id: EVENT_ID}
    const election = {
        ...scope,
        id: ELECTION_ID,
        name: "Harbour election",
        presentation: {i18n: {en: {name: "Harbour election"}}},
        keys_ceremony_id: KEYS_ID,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        labels: {},
        annotations: {},
        status: {is_published: true, voting_status: "CLOSED", allow_tally: "allowed"},
    }
    table(portal, "sequent_backend_election", () => [election])
    const candidates = [
        [ALICE_ID, "Alice Example"],
        [BOB_ID, "Bob Example"],
    ].map(([id, name]) => ({
        ...scope,
        id,
        contest_id: CONTEST_ID,
        name,
        presentation: {i18n: {en: {name}}},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }))
    const contest = {
        ...scope,
        id: CONTEST_ID,
        election_id: ELECTION_ID,
        name: "Mayor",
        presentation: {i18n: {en: {name: "Mayor"}}},
        counting_algorithm: "plurality-at-large",
        min_votes: 0,
        max_votes: 1,
        num_winners: 1,
        candidates,
        candidates_aggregate: {aggregate: {count: candidates.length}, nodes: candidates},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
    table(portal, "sequent_backend_contest", () => [contest])
    table(portal, "sequent_backend_candidate", () => candidates)
    const session = {
        ...scope,
        id: TALLY_ID,
        keys_ceremony_id: KEYS_ID,
        election_ids: [ELECTION_ID],
        area_ids: [],
        execution_status: "STARTED",
        threshold: 2,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        name: "Harbour tally",
        tally_type: "ELECTORAL_RESULTS",
        annotations: {},
        labels: {},
        configuration: {},
        permission_label: null,
        is_execution_completed: false,
        resolutions: [],
        resolutions_aggregate: {aggregate: {count: 0}, nodes: []},
    }
    const execution = {
        ...scope,
        id: EXECUTION_ID,
        tally_session_id: TALLY_ID,
        created_at: FIXED_TIME,
        current_message_id: 1,
        status: {
            trustees: TRUSTEES.map((name) => ({name, status: "WAITING"})),
            elections_status: [{election_id: ELECTION_ID, status: "WAITING", progress: 0}],
            logs: [{created_date: FIXED_TIME, log_text: "Tally ceremony started"}],
        },
        documents: {} as Row,
        results_event_id: null as string | null,
    }
    const state = {
        event,
        election,
        contest,
        session,
        execution,
        sessions: [session] as Row[],
        resolutions: [] as Row[],
    }
    table(portal, "sequent_backend_tally_session", () => state.sessions)
    table(portal, "sequent_backend_tally_session_execution", () => [execution])
    table(portal, "sequent_backend_tally_session_resolution", () => state.resolutions)
    table(portal, "sequent_backend_tally_session_contest", () => [])
    table(portal, "sequent_backend_results_event", () =>
        execution.results_event_id
            ? [
                  {
                      ...scope,
                      id: RESULTS_ID,
                      created_at: FIXED_TIME,
                      last_updated_at: FIXED_TIME,
                      documents: {},
                      annotations: {},
                  },
              ]
            : []
    )
    return state
}

/** Real SQLite bytes, read by the production portal through its own SQL.js/WASM loader. */
export async function sqlite(dataset: Record<string, Row[]>, scope: Row): Promise<Uint8Array> {
    const require = createRequire(import.meta.url)
    const sql = await initSqlJs({locateFile: (file) => require.resolve(`sql.js/dist/${file}`)})
    const db = new sql.Database()
    try {
        for (const [name, rows] of Object.entries(dataset)) {
            // Empty tables still expose every column used by the production filtered reads.
            const columns = [
                ...new Set(["id", ...Object.keys(scope), ...rows.flatMap(Object.keys)]),
            ]
            db.run(`CREATE TABLE "${name}" (${columns.map((column) => `"${column}"`).join(", ")})`)
            for (const row of rows)
                db.run(
                    `INSERT INTO "${name}" VALUES (${columns.map(() => "?").join(", ")})`,
                    columns.map((column) => {
                        const value = row[column]
                        if (value == null) return null
                        if (typeof value === "string" || typeof value === "number") return value
                        if (typeof value === "boolean") return Number(value)
                        return JSON.stringify(value)
                    })
                )
        }
        return db.export()
    } finally {
        db.close()
    }
}

/**
 * The results database of the tally world: Alice 37 and Bob 23 of 60 votes overall, and
 * in North precinct Alice 20 and Bob 10 of 30.
 */
export function resultsDataset(world: ReturnType<typeof tallyWorld>): Record<string, Row[]> {
    const scope = {
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: ELECTION_ID,
        contest_id: CONTEST_ID,
        results_event_id: RESULTS_ID,
    }
    const candidates = (world.contest.candidates as Row[]).map((candidate) => ({
        ...scope,
        id: candidate.id,
        name: candidate.name,
        presentation: candidate.presentation,
    }))
    const votes = (overall: [number, number]) =>
        candidates.map((candidate, index) => ({
            candidate_id: candidate.id,
            cast_votes: overall[index],
            cast_votes_percent: overall[index] / (overall[0] + overall[1]),
            winning_position: index === 0 ? 1 : null,
        }))
    const contestTotals = (census: number, total: number) => ({
        elegible_census: census,
        total_votes: total,
        total_votes_percent: total / census,
        total_valid_votes: total,
        total_valid_votes_percent: 1,
        total_invalid_votes: 0,
        total_invalid_votes_percent: 0,
        total_blank_votes: 0,
        total_blank_votes_percent: 0,
        documents: {},
        annotations: {},
    })
    return {
        election_event: [world.event],
        election: [world.election],
        contest: [world.contest],
        candidate: candidates,
        results_event: [{...scope, id: RESULTS_ID, documents: {}}],
        results_election: [
            {
                ...scope,
                id: "election-result",
                documents: {},
                elegible_census: 100,
                total_voters: 60,
                total_voters_percent: 0.6,
            },
        ],
        results_contest: [{...scope, id: "contest-result", ...contestTotals(100, 60)}],
        results_contest_candidate: votes([37, 23]).map((row, index) => ({
            ...scope,
            id: `candidate-result-${index}`,
            ...row,
        })),
        area: [{...scope, id: AREA_ID, name: "North precinct", parent_id: null}],
        area_contest: [{...scope, id: "area-contest", area_id: AREA_ID}],
        results_area_contest: [
            {...scope, id: "area-result", area_id: AREA_ID, ...contestTotals(50, 30)},
        ],
        results_area_contest_candidate: votes([20, 10]).map((row, index) => ({
            ...scope,
            id: `area-candidate-result-${index}`,
            area_id: AREA_ID,
            ...row,
        })),
        results_election_area: [
            {
                ...scope,
                id: "election-area-result",
                area_id: AREA_ID,
                documents: {},
                name: "North precinct",
            },
        ],
    }
}

/** Serves the results database through FetchDocument and the private bucket. */
export async function serveResults(portal: AdminPortal, world: ReturnType<typeof tallyWorld>) {
    const bytes = await sqlite(resultsDataset(world), {
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: ELECTION_ID,
        contest_id: CONTEST_ID,
        results_event_id: RESULTS_ID,
    })
    const key = `${TENANT_ID}/${EVENT_ID}/results.sqlite`
    portal.s3.putBytes("private", key, bytes, "application/vnd.sqlite3")
    const url = portal.s3.presign(key, "results")
    portal.graphql.on("FetchDocument", ({variables}) =>
        variables.documentId === SQLITE_DOCUMENT_ID
            ? {data: {fetchDocument: {url}}}
            : {errors: [{message: `unknown document ${String(variables.documentId)}`}]}
    )
    world.execution.results_event_id = RESULTS_ID
    world.execution.documents = {sqlite: SQLITE_DOCUMENT_ID}
    return key
}

export const SQLITE_DOCUMENT_ID = "62000000-0000-4000-8000-000000000001"
