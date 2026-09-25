// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect, TENANT_ID} from "./fixtures"
import type {AdminPortal} from "./fixtures"
import {createRequire} from "node:module"
import initSqlJs from "sql.js"

const EVENT_ID = "20000000-0000-4000-8000-000000000001"
const ELECTION_ID = "30000000-0000-4000-8000-000000000001"
const KEYS_ID = "40000000-0000-4000-8000-000000000001"
const FIXED_TIME = "2026-01-01T00:00:00Z"
const roles = [
    "admin-user",
    "election-event-read",
    "election-read",
    "admin-ceremony",
    "create-ceremony",
    "keys-read",
    "election-event-keys-tab",
    "election-event-keys-columns",
    "tally-read",
    "tally-start",
    "tally-write",
    "tally-results-read",
    "publish-results-read",
    "publish-results-write",
    "election-event-tally-tab",
    "election-event-tally-columns",
]
test.use({roles})

function registerEvent(portal: AdminPortal) {
    const event = {
        id: EVENT_ID,
        tenant_id: TENANT_ID,
        name: "Council event",
        alias: "Council event",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        labels: {},
        annotations: {},
        is_archived: false,
        encryption_protocol: "RSA256",
        presentation: {
            i18n: {en: {name: "Council event"}},
            language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
            ceremonies_policy: "automated-ceremonies",
            results_website: {status: "enabled", access: "public", visibility_scope: "full_event"},
        },
        status: {is_published: true, voting_status: "CLOSED"},
    }
    portal.graphql.on("sequent_backend_election_event", () => ({
        data: {
            sequent_backend_election_event: [event],
            sequent_backend_election_event_by_pk: event,
            sequent_backend_election_event_aggregate: {aggregate: {count: 1}},
        },
    }))
    portal.graphql.on("election_events_tree", () => ({data: {sequent_backend_election_event: []}}))
    portal.graphql.on("election_tree", () => ({data: {sequent_backend_election: []}}))
    const trustees = ["Alice", "Bob"].map((name, index) => ({
        id: `50000000-0000-4000-8000-00000000000${index + 1}`,
        name,
        tenant_id: TENANT_ID,
        labels: {},
        annotations: {},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }))
    portal.graphql.on("TrusteeNames", () => ({data: {sequent_backend_trustee: trustees}}))
    portal.graphql.on("sequent_backend_trustee", () => ({
        data: {
            sequent_backend_trustee: trustees,
            sequent_backend_trustee_aggregate: {aggregate: {count: 2}},
        },
    }))
    return event
}

test("automatic keys ceremony completes with the selected trustee threshold", async ({
    page,
    portal,
}) => {
    registerEvent(portal)
    let created = false
    portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
    const ceremony = {
        id: KEYS_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        trustee_ids: ["Alice", "Bob"],
        status: {
            public_key: "synthetic-public-key",
            trustees: [
                {name: "Alice", status: "KEY_CHECKED"},
                {name: "Bob", status: "KEY_CHECKED"},
            ],
            logs: [],
        },
        execution_status: "STARTED",
        labels: {},
        annotations: {},
        threshold: 2,
        name: "Council key",
        settings: {policy: "automated-ceremonies"},
        is_default: true,
        permission_label: null,
    }
    portal.graphql.on("ListKeysCeremony", () => ({
        data: {
            list_keys_ceremony: {
                items: created ? [ceremony] : [],
                total: {aggregate: {count: created ? 1 : 0}},
            },
        },
    }))
    portal.graphql.on("sequent_backend_keys_ceremony", () => ({
        data: {
            sequent_backend_keys_ceremony: created ? [ceremony] : [],
            sequent_backend_keys_ceremony_by_pk: created ? ceremony : null,
            sequent_backend_keys_ceremony_aggregate: {aggregate: {count: created ? 1 : 0}},
        },
    }))
    portal.graphql.on("sequent_backend_election", () => ({
        data: {
            sequent_backend_election: [],
            sequent_backend_election_aggregate: {aggregate: {count: 0}},
        },
    }))
    portal.graphql.on("CreateKeysCeremony", () => {
        created = true
        return {data: {create_keys_ceremony: {keys_ceremony_id: KEYS_ID, error_message: null}}}
    })
    await page.goto(`${portal.origin}/sequent_backend_election_event/${EVENT_ID}?lang=en`)
    await page.getByRole("button", {name: /Create Key Ceremony$/}).click()
    await page.getByRole("checkbox", {name: "Alice", exact: true}).check()
    await page.getByRole("checkbox", {name: "Bob", exact: true}).check()
    await page.getByRole("switch", {name: "Automatic Ceremony"}).check()
    await page.getByRole("button", {name: "Create Key Ceremony", exact: true}).click()
    const dialog = page.getByRole("dialog")
    await expect(
        dialog.getByText("Are you sure you want to Create Automatic Key Ceremony?")
    ).toBeVisible()
    await dialog.getByRole("button", {name: "Yes, Create Key Ceremony"}).click()
    await expect(page.getByText("Status: STARTED", {exact: true})).toBeVisible()
    ceremony.execution_status = "SUCCESS"
    await page.clock.runFor(101)
    await expect(page.getByText("Status: SUCCESS", {exact: true})).toBeVisible()
    await expect(page.getByRole("rowheader", {name: "Alice"})).toBeVisible()
    await expect(page.getByRole("rowheader", {name: "Bob"})).toBeVisible()
    const calls = portal.graphql.callsTo("CreateKeysCeremony")
    expect(calls).toHaveLength(1)
    expect(calls[0].variables).toEqual({
        electionEventId: EVENT_ID,
        threshold: 2,
        trusteeNames: ["Alice", "Bob"],
        electionId: null,
        name: "All Elections",
        isAutomaticCeremony: true,
    })
    expect(calls[0].headers["x-hasura-role"]).toBe("admin-ceremony")
})

const TALLY_ID = "60000000-0000-4000-8000-000000000001"
const EXECUTION_ID = "60000000-0000-4000-8000-000000000002"
const RESULTS_ID = "60000000-0000-4000-8000-000000000003"
const CONTEST_ID = "70000000-0000-4000-8000-000000000001"

function table(portal: AdminPortal, name: string, records: () => Record<string, unknown>[]) {
    portal.graphql.on(name, ({variables}) => {
        const rows = records()
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

test("a closed published election advances from automatic tally to results publication", async ({
    page,
    portal,
}) => {
    const event = registerEvent(portal)
    portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
    portal.settings.QUERY_POLL_INTERVAL_MS = 100
    const keys = {
        id: KEYS_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        trustee_ids: ["Alice", "Bob"],
        status: {public_key: "synthetic-public-key", trustees: [], logs: []},
        execution_status: "SUCCESS",
        labels: {},
        annotations: {},
        threshold: 2,
        name: "Council key",
        settings: {policy: "automated-ceremonies"},
        is_default: true,
        permission_label: null,
    }
    portal.graphql.on("ListKeysCeremony", () => ({
        data: {
            list_keys_ceremony: {
                items: [keys],
                total: {aggregate: {count: 1}},
            },
        },
    }))
    table(portal, "sequent_backend_keys_ceremony", () => [keys])
    const election = {
        id: ELECTION_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name: "Council election",
        presentation: {i18n: {en: {name: "Council election"}}},
        keys_ceremony_id: KEYS_ID,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        labels: {},
        annotations: {},
        status: {
            is_published: true,
            voting_status: "CLOSED",
            allow_tally: "requires-voting-period-end",
        },
    }
    const contest = {
        id: CONTEST_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: ELECTION_ID,
        name: "Council representative",
        presentation: {i18n: {en: {name: "Council representative"}}},
        counting_algorithm: "plurality-at-large",
        min_votes: 0,
        max_votes: 1,
        num_winners: 1,
        candidates: [],
        candidates_aggregate: {aggregate: {count: 0}, nodes: []},
    }
    table(portal, "sequent_backend_election", () => [election])
    table(portal, "sequent_backend_contest", () => [contest])
    let created = false
    const session = {
        id: TALLY_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        keys_ceremony_id: KEYS_ID,
        election_ids: [ELECTION_ID],
        execution_status: "IN_PROGRESS",
        threshold: 2,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        name: "Council tally",
        tally_type: "ELECTORAL_RESULTS",
        annotations: {},
        labels: {},
        configuration: {},
        is_execution_completed: false,
    }
    const execution = {
        id: EXECUTION_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        tally_session_id: TALLY_ID,
        created_at: FIXED_TIME,
        current_message_id: 1,
        status: {
            trustees: [],
            elections_status: [{election_id: ELECTION_ID, status: "IN_PROGRESS", progress: 25}],
            logs: [],
        },
        documents: {} as Record<string, string>,
        results_event_id: null as string | null,
    }
    table(portal, "sequent_backend_tally_session", () => (created ? [session] : []))
    table(portal, "sequent_backend_tally_session_execution", () => (created ? [execution] : []))
    const resultsEvent = {
        id: RESULTS_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        documents: {},
        annotations: {},
    }
    table(portal, "sequent_backend_results_event", () =>
        execution.results_event_id ? [resultsEvent] : []
    )
    table(portal, "sequent_backend_tally_session_resolution", () => [])
    table(portal, "sequent_backend_tally_session_contest", () => [])
    const bytes = await resultsDatabase(event, election, contest, resultsEvent)
    const documentId = "80000000-0000-4000-8000-000000000001"
    const objectKey = `${TENANT_ID}/${EVENT_ID}/results.sqlite`
    portal.s3.putBytes("private", objectKey, bytes, "application/vnd.sqlite3")
    const documentUrl = portal.s3.presign(objectKey, "synthetic-results-signature")
    portal.graphql.on("FetchDocument", () => ({data: {fetchDocument: {url: documentUrl}}}))
    const publicationId = "80000000-0000-4000-8000-000000000002"
    const taskId = "80000000-0000-4000-8000-000000000003"
    let published = false
    table(portal, "sequent_backend_tally_results_publication", () =>
        published
            ? [
                  {
                      id: publicationId,
                      tenant_id: TENANT_ID,
                      election_event_id: EVENT_ID,
                      tally_session_id: TALLY_ID,
                      tally_session_execution_id: EXECUTION_ID,
                      results_event_id: RESULTS_ID,
                      version: 1,
                      publication_status: "Published",
                      route_scope: "election",
                      route_election_id: ELECTION_ID,
                      election_ids: [ELECTION_ID],
                      published_contest_ids: [CONTEST_ID],
                      contest_publication_state: {},
                      access: "public",
                      visibility_scope: "full_event",
                      documents: {},
                      created_at: FIXED_TIME,
                      updated_at: FIXED_TIME,
                      published_at: FIXED_TIME,
                      revoked_at: null,
                  },
              ]
            : []
    )
    portal.graphql.on("PublishResultsWebsite", () => {
        published = true
        return {
            data: {
                publishResultsWebsite: {
                    publication_id: publicationId,
                    task_execution_id: taskId,
                    publication_status: "Published",
                    error_msg: null,
                },
            },
        }
    })
    portal.graphql.on("GetTaskById", () => ({
        data: {
            sequent_backend_tasks_execution: [
                {
                    id: taskId,
                    tenant_id: TENANT_ID,
                    election_event_id: EVENT_ID,
                    type: "PUBLISH_RESULTS_WEBSITE",
                    execution_status: "SUCCESS",
                    start_at: FIXED_TIME,
                    end_at: FIXED_TIME,
                    logs: [],
                    annotations: {},
                    executed_by_user: "90000000-0000-4000-8000-000000000001",
                },
            ],
        },
    }))
    portal.graphql.on("CreateTallyCeremony", () => {
        created = true
        return {data: {create_tally_ceremony: {tally_session_id: TALLY_ID}}}
    })
    await page.goto(`${portal.origin}/sequent_backend_election_event/${EVENT_ID}?lang=en`)
    await page.getByRole("tab", {name: "Tally", exact: true}).click()
    await expect(page.getByRole("button", {name: /Start Tally Ceremony/})).toBeEnabled()
    await page.getByRole("button", {name: /Start Tally Ceremony/}).click()
    await expect(page.getByRole("row", {name: /Council election/})).toBeVisible()
    await page.getByRole("button", {name: "Start Tally", exact: true}).click()
    const dialog = page.getByRole("dialog")
    await expect(
        dialog.getByText(
            "Select Start Tally to run tally process and display results, or Close to cancel."
        )
    ).toBeVisible()
    expect(portal.graphql.callsTo("CreateTallyCeremony")).toEqual([])
    await dialog.getByRole("button", {name: "Ok", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("CreateTallyCeremony").length).toBe(1)
    expect(portal.graphql.callsTo("CreateTallyCeremony")[0].variables).toEqual({
        election_event_id: EVENT_ID,
        election_ids: [ELECTION_ID],
        tally_type: "ELECTORAL_RESULTS",
    })
    await expect(page.getByText("Status: IN_PROGRESS", {exact: true})).toBeVisible()
    execution.results_event_id = RESULTS_ID
    execution.documents = {sqlite: documentId}
    execution.status.elections_status[0] = {
        election_id: ELECTION_ID,
        status: "SUCCESS",
        progress: 100,
    }
    session.execution_status = "SUCCESS"
    session.is_execution_completed = true
    await page.clock.runFor(101)
    await expect(page.getByText("Status: SUCCESS", {exact: true})).toBeVisible()
    await expect(
        page
            .getByRole("row", {name: /Alice Example/})
            .getByRole("gridcell", {name: "37", exact: true})
    ).toBeVisible()
    await expect(
        page
            .getByRole("row", {name: /Bob Example/})
            .getByRole("gridcell", {name: "23", exact: true})
    ).toBeVisible()
    const downloads = portal.s3.requestsFor(objectKey)
    expect(downloads).toHaveLength(1)
    expect(downloads[0]).toMatchObject({method: "GET", status: 200, bucket: "private"})
    expect(portal.graphql.callsTo("FetchDocument")[0].variables).toEqual({
        electionEventId: EVENT_ID,
        documentId,
    })
    await page.getByRole("button", {name: "Publish to results website", exact: true}).click()
    await expect(
        page.getByRole("checkbox", {name: "Council election - Council representative"})
    ).toBeChecked()
    await page.getByRole("button", {name: "Publish selected contests", exact: true}).click()
    await expect(
        dialog.getByRole("heading", {name: "Start publish to results website?"})
    ).toBeVisible()
    expect(portal.graphql.callsTo("PublishResultsWebsite")).toEqual([])
    await dialog.getByRole("button", {name: "Publish selected contests"}).click()
    const publicationRow = page
        .getByRole("row")
        .filter({has: page.getByRole("link", {name: "Open", exact: true})})
    await expect(publicationRow).toContainText("Published")
    await expect(publicationRow.getByRole("link", {name: "Open", exact: true})).toHaveAttribute(
        "href",
        `${portal.origin}/results/${EVENT_ID}/elections/${ELECTION_ID}`
    )
    const publishCalls = portal.graphql.callsTo("PublishResultsWebsite")
    expect(publishCalls).toHaveLength(1)
    expect(publishCalls[0].variables).toEqual({
        election_event_id: EVENT_ID,
        tally_session_id: TALLY_ID,
        tally_session_execution_id: EXECUTION_ID,
        results_event_id: RESULTS_ID,
        route_scope: "election",
        route_election_id: ELECTION_ID,
        election_ids: [ELECTION_ID],
        contest_ids: [CONTEST_ID],
        access: "public",
        visibility_scope: "full_event",
    })
    expect(publishCalls[0].headers["x-hasura-role"]).toBe("publish-results-write")
    await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
    expect(portal.graphql.callsTo("GetTaskById")[0].variables).toEqual({task_id: taskId})
})

/** Real SQLite bytes, loaded by the production portal's own SQL.js/WASM path. */
async function resultsDatabase(
    event: Record<string, unknown>,
    election: Record<string, unknown>,
    contest: Record<string, unknown>,
    resultsEvent: Record<string, unknown>
): Promise<Uint8Array> {
    const scope = {
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: ELECTION_ID,
        contest_id: CONTEST_ID,
        results_event_id: RESULTS_ID,
    }
    const candidates = ["Alice Example", "Bob Example"].map((name, index) => ({
        ...scope,
        id: `90000000-0000-4000-8000-00000000000${index + 1}`,
        name,
        presentation: {i18n: {en: {name}}},
    }))
    const dataset: Record<string, Record<string, unknown>[]> = {
        election_event: [event],
        election: [election],
        contest: [contest],
        candidate: candidates,
        results_event: [resultsEvent],
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
        results_contest: [
            {
                ...scope,
                id: "contest-result",
                documents: {},
                annotations: {},
                elegible_census: 100,
                total_votes: 60,
                total_votes_percent: 0.6,
                total_valid_votes: 60,
                total_valid_votes_percent: 1,
                total_invalid_votes: 0,
                total_invalid_votes_percent: 0,
                total_blank_votes: 0,
                total_blank_votes_percent: 0,
            },
        ],
        results_contest_candidate: candidates.map((candidate, index) => ({
            ...scope,
            id: `candidate-result-${index}`,
            candidate_id: candidate.id,
            cast_votes: index === 0 ? 37 : 23,
            cast_votes_percent: index === 0 ? 37 / 60 : 23 / 60,
            winning_position: index === 0 ? 1 : null,
        })),
        area: [],
        area_contest: [],
        results_area_contest: [],
        results_area_contest_candidate: [],
        results_election_area: [],
    }
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
            for (const row of rows) {
                const values = columns.map((column) => {
                    const value = row[column]
                    if (value == null) return null
                    if (typeof value === "string" || typeof value === "number") return value
                    if (typeof value === "boolean") return Number(value)
                    return JSON.stringify(value)
                })
                db.run(
                    `INSERT INTO "${name}" VALUES (${columns.map(() => "?").join(", ")})`,
                    values
                )
            }
        }
        return db.export()
    } finally {
        db.close()
    }
}
