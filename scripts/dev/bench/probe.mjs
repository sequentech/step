// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Browser probe for the ui-update benchmark. It keeps one headless Chromium page
// per target open on a running dev server and reports, as JSON lines on stdout,
// the epoch time at which a text becomes visible or disappears. Commands arrive
// as JSON lines on stdin:
//   {"cmd": "open", "id": "voting", "kind": "voting", "origin": "http://127.0.0.1:41000"}
//   {"cmd": "wait", "id": "voting", "text": "bench...", "timeout": 600}
//   {"cmd": "gone", "id": "voting", "text": "bench...", "timeout": 600}
//   {"cmd": "park", "id": "voting"}   (leave the page while its server restarts)
//   {"cmd": "visit", "id": "voting", "text": "bench...", "timeout": 600}
//   {"cmd": "close"}
// "visit" loads the page again and reports once it rendered and loaded a WASM
// module whose bytes contain the text.
// Portal pages authenticate against the ui-test-kit OIDC, GraphQL and S3 mocks
// of the measured checkout. Unlike the journeys' strict adapter, the dev
// server's live-reload socket and assets reach the server unrouted; only the
// mocked paths are answered locally and other origins are blocked.
import {register, createRequire} from "node:module"
import {createInterface} from "node:readline"
import {parseArgs} from "node:util"

const {values: options} = parseArgs({
    options: {packages: {type: "string"}, chromium: {type: "string"}},
})
const packages = options.packages
if (!packages) throw new Error("--packages <checkout>/packages is required")
register("./typescript-hooks.mjs", import.meta.url, {data: {packages}})

const {chromium} = createRequire(`${packages}/package.json`)("@playwright/test")
const kit = async (module) => import(`${packages}/ui-test-kit/${module}.ts`)
const {GraphQLMock, loadClientSchema} = await kit("mocks/graphql")
const {OidcMock} = await kit("mocks/oidc")
const {S3Mock} = await kit("mocks/s3")
const {ViolationLog} = await kit("mocks/violations")
const {MOCK_PATHS} = await kit("mocks/http")
const {electionFixture, IDS} = await kit("fixtures/index")

function emit(event) {
    process.stdout.write(`${JSON.stringify({...event, t: Date.now() / 1000})}\n`)
}

function mocks(origin, portal) {
    const violations = new ViolationLog()
    const s3 = new S3Mock({origin, violations})
    const graphql = new GraphQLMock({
        schema: loadClientSchema(`${packages}/${portal}/graphql.schema.json`),
        violations,
    })
    return {violations, s3, graphql}
}

function commonSettings(origin, oidc, s3) {
    return {
        KEYCLOAK_URL: oidc.serverUrl(),
        HASURA_URL: `${origin}/v1/graphql`,
        PUBLIC_BUCKET_URL: s3.publicBucketUrl(),
        APP_VERSION: "bench",
        APP_HASH: "bench",
    }
}

// Each setup mirrors the portal's journey fixture closely enough to render the
// first authenticated screen: the start path and a text proving it rendered.
const setups = {
    async voting(origin) {
        const {violations, s3, graphql} = mocks(origin, "voting-portal")
        const oidc = new OidcMock({
            origin,
            violations,
            realms: [
                {
                    name: `tenant-${IDS.tenant}-event-${IDS.event}`,
                    clients: ["voting-portal", "voting-portal-kiosk"],
                    user: {
                        id: IDS.voter,
                        username: "synthetic-voter",
                        attributes: {"tenant-id": [IDS.tenant]},
                    },
                    claims: () => ({
                        "https://hasura.io/jwt/claims": {
                            "x-hasura-default-role": "voter",
                            "x-hasura-allowed-roles": ["voter"],
                            "x-hasura-tenant-id": IDS.tenant,
                            "x-hasura-election-event-id": IDS.event,
                            "x-hasura-user-id": IDS.voter,
                        },
                    }),
                },
            ],
        })
        const data = electionFixture()
        const {event, election, style, summary} = data
        s3.putJson("public", `tenant-${IDS.tenant}/event-${IDS.event}/election_event_config.json`, {
            id: IDS.event,
            tenant_id: IDS.tenant,
            election_event_id: IDS.event,
            election_event_presentation: event.presentation,
        })
        for (const [key, document] of Object.entries({event, election, style, summary}))
            s3.putJson("private", `${key}.json`, document)
        graphql.on("GetVoterStatus", () => ({
            data: {
                get_ballot_files_urls: {
                    event_id: IDS.event,
                    status: event.status,
                    files: [
                        {
                            id: IDS.style,
                            election_id: IDS.election,
                            version: "1",
                            status: election.status,
                            num_allowed_revotes: election.num_allowed_revotes,
                            voting_channels: election.voting_channels,
                            urls: {
                                event_url: s3.presign("event.json", "bench"),
                                election_url: s3.presign("election.json", "bench"),
                                summary_url: s3.presign("summary.json", "bench"),
                                style_url: s3.presign("style.json", "bench"),
                            },
                        },
                    ],
                },
                sequent_backend_cast_vote: [],
            },
        }))
        return {
            path: `/tenant/${IDS.tenant}/event/${IDS.event}?lang=en`,
            ready: "Community Council",
            services: {origin, violations, s3, graphql, oidc},
            settings: {
                ...commonSettings(origin, oidc, s3),
                DISABLE_AUTH: false,
                QUERY_POLL_INTERVAL_MS: 60000,
                DEFAULT_TENANT_ID: IDS.tenant,
                DEFAULT_EVENT_ID: IDS.event,
                ONLINE_VOTING_CLIENT_ID: "voting-portal",
                BALLOT_VERIFIER_URL: `${origin}/verifier`,
                RESULTS_PORTAL_URL: `${origin}/results`,
                KEYCLOAK_ACCESS_TOKEN_LIFESPAN_SECS: 900,
                POLLING_DURATION_TIMEOUT: 12000,
            },
        }
    },

    async admin(origin) {
        const {violations, s3, graphql} = mocks(origin, "admin-portal")
        const oidc = new OidcMock({
            origin,
            violations,
            realms: [
                {
                    name: `tenant-${IDS.tenant}`,
                    clients: ["admin-portal"],
                    user: {
                        id: IDS.voter,
                        username: "synthetic-admin",
                        attributes: {"tenant-id": [IDS.tenant]},
                    },
                    claims: () => ({
                        "https://hasura.io/jwt/claims": {
                            "x-hasura-default-role": "admin-user",
                            "x-hasura-allowed-roles": [
                                "admin-user",
                                "election-event-read",
                                "election-event-write",
                                "election-event-create",
                                "election-read",
                            ],
                            "x-hasura-tenant-id": IDS.tenant,
                            "x-hasura-user-id": IDS.voter,
                        },
                    }),
                },
            ],
        })
        const tenant = {
            id: IDS.tenant,
            slug: "synthetic",
            annotations: {},
            settings: {language_conf: {enabled_language_codes: ["en"], default_language_code: "en"}},
            is_active: true,
            labels: {},
            voting_channels: ["ONLINE"],
            created_at: "2026-01-15T12:00:00.000Z",
            updated_at: "2026-01-15T12:00:00.000Z",
            test: true,
        }
        graphql.on("IntrospectionQuery", () => ({data: {}}))
        graphql.on("GetTenantBySlug", () => ({data: {sequent_backend_tenant: [tenant]}}))
        graphql.on("GetTenantById", () => ({data: {sequent_backend_tenant: [tenant]}}))
        graphql.on("sequent_backend_tenant", () => ({
            data: {
                sequent_backend_tenant: [tenant],
                sequent_backend_tenant_aggregate: {aggregate: {count: 1}},
            },
        }))
        graphql.on("election_events_tree", () => ({data: {sequent_backend_election_event: []}}))
        graphql.on("sequent_backend_election_event", () => ({
            data: {
                sequent_backend_election_event: [],
                sequent_backend_election_event_aggregate: {aggregate: {count: 0}},
            },
        }))
        return {
            path: "/?lang=en",
            ready: "No Election Event yet",
            services: {origin, violations, s3, graphql, oidc},
            settings: {
                ...commonSettings(origin, oidc, s3),
                QUERY_POLL_INTERVAL_MS: 3_600_000,
                QUERY_FAST_POLL_INTERVAL_MS: 3_600_000,
                DEFAULT_TENANT_ID: IDS.tenant,
                ONLINE_VOTING_CLIENT_ID: "admin-portal",
                VOTING_PORTAL_URL: `${origin}/voting`,
                RESULTS_PORTAL_URL: `${origin}/results`,
            },
        }
    },

    async verifier(origin) {
        const {violations, s3, graphql} = mocks(origin, "ballot-verifier")
        const oidc = new OidcMock({
            origin,
            violations,
            realms: [
                {
                    name: `tenant-${IDS.tenant}-event-${IDS.event}`,
                    clients: ["ballot-verifier"],
                    user: {
                        id: IDS.voter,
                        username: "synthetic-voter",
                        attributes: {"tenant-id": [IDS.tenant]},
                    },
                },
            ],
        })
        s3.putJson("public", `tenant-${IDS.tenant}/event-${IDS.event}/election_event_config.json`, {
            id: IDS.event,
            tenant_id: IDS.tenant,
            election_event_id: IDS.event,
            election_event_presentation: electionFixture().event.presentation,
        })
        graphql.on("GetBallotStyles", () => ({
            data: {sequent_backend_ballot_publication: [], sequent_backend_ballot_style: []},
        }))
        return {
            path: `/tenant/${IDS.tenant}/event/${IDS.event}/start?lang=en`,
            ready: "Step 1: Import your ballot",
            services: {origin, violations, s3, graphql, oidc},
            settings: {
                ...commonSettings(origin, oidc, s3),
                DISABLE_AUTH: false,
                QUERY_POLL_INTERVAL_MS: 60000,
                DEFAULT_TENANT_ID: IDS.tenant,
                DEFAULT_EVENT_ID: IDS.event,
                ONLINE_VOTING_CLIENT_ID: "ballot-verifier",
            },
        }
    },

    async results(origin) {
        const {resultsFixture, resultsIds} = await import(
            `${packages}/results-portal/tests/fixtures/results.ts`
        )
        const {sqliteBytes} = await import(`${packages}/results-portal/tests/fixtures/sqlite.ts`)
        const {violations, s3, graphql} = mocks(origin, "results-portal")
        const oidc = new OidcMock({origin, violations, realms: []})
        const data = resultsFixture()
        s3.putJson("public", `results-index/${resultsIds.event}.json`, data.index)
        s3.putJson("public", "results/manifest.json", data.manifest)
        const bytes = await sqliteBytes(data.dataset)
        s3.putBytes("public", "results/full.sqlite", bytes, "application/vnd.sqlite3")
        return {
            path: `/${resultsIds.event}?lang=en`,
            ready: "Community Election Results",
            services: {origin, violations, s3, graphql, oidc},
            settings: {...commonSettings(origin, oidc, s3), RESULTS_PORTAL_CLIENT_ID: "results-portal"},
        }
    },
}

// Storybook needs no services: it renders one story in its preview iframe.
const STORYBOOK = {
    path: "/iframe.html?id=components-header--primary&viewMode=story",
    readySelector: "[role=banner]",
}

async function routeServices(context, origin, setup) {
    const {services, settings} = setup
    const mocked = (url) =>
        url.origin === origin &&
        (url.pathname === "/global-settings.json" ||
            url.pathname === MOCK_PATHS.graphql ||
            services.oidc.handles(url) ||
            services.s3.handles(url))
    await context.route(
        (url) => url.origin !== origin,
        (route) => route.abort("blockedbyclient")
    )
    await context.route(mocked, async (route) => {
        const request = route.request()
        const url = new URL(request.url())
        const mockRequest = {
            method: request.method(),
            url,
            headers: await request.allHeaders(),
            body: request.postData() ?? undefined,
        }
        let response
        if (url.pathname === "/global-settings.json")
            response = {
                status: 200,
                headers: {"content-type": "application/json"},
                body: JSON.stringify(settings),
            }
        else if (url.pathname === MOCK_PATHS.graphql)
            response = await services.graphql.handle(mockRequest)
        else if (services.oidc.handles(url)) response = services.oidc.handle(mockRequest)
        else response = services.s3.handle(mockRequest)
        if ("abort" in response) await route.abort(response.abort)
        else
            await route.fulfill({
                ...response,
                body: response.body instanceof Uint8Array ? Buffer.from(response.body) : response.body,
            })
    })
}

const browser = await chromium.launch({
    executablePath: options.chromium || process.env.CHROMIUM_EXECUTABLE_PATH || undefined,
    chromiumSandbox: false,
})
const pages = new Map()

async function open({id, kind, origin, timeout = 600}) {
    const context = await browser.newContext({
        locale: "en-US",
        timezoneId: "UTC",
        viewport: {width: 1280, height: 800},
        serviceWorkers: "block",
    })
    const page = await context.newPage()
    const state = {page, loads: 0, errors: 0}
    page.on("load", () => (state.loads += 1))
    page.on("pageerror", () => (state.errors += 1))
    let path = STORYBOOK.path
    let ready = page.locator(STORYBOOK.readySelector)
    if (kind !== "storybook") {
        const setup = await setups[kind](origin)
        await routeServices(context, origin, setup)
        path = setup.path
        ready = page.getByText(setup.ready, {exact: true}).first()
        state.violations = setup.services.violations
    }
    state.ready = ready
    state.url = `${origin}${path}`
    pages.set(id, state)
    await page.goto(state.url, {timeout: timeout * 1000})
    await ready.waitFor({state: "visible", timeout: timeout * 1000})
    emit({event: "opened", id})
}

// "gone" also waits for the page to render again, since a reload removes the
// marker before the rebuilt page shows the restored content.
async function wait({cmd, id, text, timeout = 600}) {
    const state = pages.get(id)
    const loads = state.loads
    const locator = state.page.getByText(text).first()
    const options = {timeout: timeout * 1000}
    emit({event: "waiting", id, text})
    if (cmd === "wait") await locator.waitFor({...options, state: "visible"})
    else {
        await locator.waitFor({...options, state: "detached"})
        await state.ready.waitFor({...options, state: "visible"})
    }
    emit({
        event: cmd === "wait" ? "visible" : "gone",
        id,
        text,
        reloads: state.loads - loads,
        page_errors: state.errors,
        violations: state.violations ? state.violations.list().length : 0,
    })
}

async function park({id}) {
    await pages.get(id).page.goto("about:blank")
    emit({event: "parked", id})
}

async function visit({id, text, timeout = 600}) {
    const state = pages.get(id)
    const loads = state.loads
    const expected = text ? Buffer.from(text) : null
    const modules = []
    const collect = (response) => {
        if (new URL(response.url()).pathname.endsWith(".wasm"))
            modules.push(response.body().catch(() => Buffer.alloc(0)))
    }
    const deadline = Date.now() + timeout * 1000
    state.page.on("response", collect)
    try {
        await state.page.goto(state.url, {timeout: timeout * 1000})
        await state.ready.waitFor({state: "visible", timeout: timeout * 1000})
        for (;;) {
            const bodies = await Promise.all(modules)
            if (bodies.some((body) => (expected ? body.includes(expected) : body.length > 0))) break
            if (Date.now() > deadline)
                throw new Error(`no WASM module with ${text} among ${bodies.length} loaded`)
            await new Promise((resolve) => setTimeout(resolve, 50))
        }
    } finally {
        state.page.off("response", collect)
    }
    emit({event: "visited", id, text, wasm_modules: modules.length, reloads: state.loads - loads})
}

const handlers = {open, wait, gone: wait, park, visit}
const lines = createInterface({input: process.stdin})
for await (const line of lines) {
    if (!line.trim()) continue
    const message = JSON.parse(line)
    if (message.cmd === "close") break
    // Waits for several targets run concurrently; each reports on its own line.
    handlers[message.cmd](message).catch((error) =>
        emit({event: "error", id: message.id, cmd: message.cmd, message: String(error)})
    )
}
await browser.close()
