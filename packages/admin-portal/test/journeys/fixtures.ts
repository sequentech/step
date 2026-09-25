// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test as base, expect} from "@playwright/test"
import {resolve, dirname} from "node:path"
import {fileURLToPath} from "node:url"
import {serveDist} from "@sequentech/ui-test-kit/server/static"
import {routePortal, type PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {GraphQLMock, loadClientSchema} from "@sequentech/ui-test-kit/mocks/graphql"
import {OidcMock} from "@sequentech/ui-test-kit/mocks/oidc"
import {S3Mock} from "@sequentech/ui-test-kit/mocks/s3"
import {ViolationLog} from "@sequentech/ui-test-kit/mocks/violations"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"

const directory = resolve(dirname(fileURLToPath(import.meta.url)), "../..")
export const TENANT_ID = IDS.tenant
export const test = base.extend<
    {portal: PortalServices; roles: string[]},
    {dist: Awaited<ReturnType<typeof serveDist>>}
>({
    roles: [
        [
            "admin-user",
            "election-event-read",
            "election-event-write",
            "election-event-create",
            "election-read",
        ],
        {option: true},
    ],
    dist: [
        // Playwright requires an object pattern for a fixture without dependencies.
        // eslint-disable-next-line no-empty-pattern
        async ({}, use) => {
            const dist = await serveDist(resolve(directory, "dist"))
            try {
                await use(dist)
            } finally {
                await dist.close()
            }
        },
        {scope: "worker"},
    ],
    portal: async ({context, page, dist, roles}, use) => {
        const {origin} = dist
        const violations = new ViolationLog()
        const s3 = new S3Mock({origin, violations})
        const graphql = new GraphQLMock({
            schema: loadClientSchema(resolve(directory, "graphql.schema.json")),
            violations,
        })
        const oidc = new OidcMock({
            origin,
            violations,
            now: () => Date.parse(FIXED_TIME),
            realms: [
                {
                    name: `tenant-${TENANT_ID}`,
                    clients: ["admin-portal"],
                    user: {
                        id: IDS.voter,
                        username: "synthetic-admin",
                        attributes: {"tenant-id": [TENANT_ID]},
                    },
                    claims: () => ({
                        "https://hasura.io/jwt/claims": {
                            "x-hasura-default-role": "admin-user",
                            "x-hasura-allowed-roles": roles,
                            "x-hasura-tenant-id": TENANT_ID,
                            "x-hasura-user-id": IDS.voter,
                        },
                    }),
                },
            ],
        })
        const portal: PortalServices = {
            origin,
            violations,
            s3,
            graphql,
            oidc,
            settings: {
                QUERY_POLL_INTERVAL_MS: 3_600_000,
                QUERY_FAST_POLL_INTERVAL_MS: 3_600_000,
                DEFAULT_TENANT_ID: TENANT_ID,
                ONLINE_VOTING_CLIENT_ID: "admin-portal",
                KEYCLOAK_URL: oidc.serverUrl(),
                HASURA_URL: `${origin}/v1/graphql`,
                PUBLIC_BUCKET_URL: s3.publicBucketUrl(),
                APP_VERSION: "test",
                APP_HASH: "test",
                VOTING_PORTAL_URL: `${origin}/voting`,
                RESULTS_PORTAL_URL: `${origin}/results`,
            },
        }
        graphql.on("IntrospectionQuery", () => ({data: {}}))
        const tenant = {
            id: TENANT_ID,
            slug: "synthetic",
            annotations: {},
            settings: {
                language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
            },
            is_active: true,
            labels: {},
            voting_channels: ["ONLINE"],
            created_at: FIXED_TIME,
            updated_at: FIXED_TIME,
            test: true,
        }
        graphql.on("GetTenantById", () => ({data: {sequent_backend_tenant: [tenant]}}))
        graphql.on("sequent_backend_tenant", () => ({data: {sequent_backend_tenant: [tenant]}}))
        graphql.on("election_events_tree", () => ({data: {sequent_backend_election_event: []}}))
        graphql.on("sequent_backend_election_event", () => ({
            data: {
                sequent_backend_election_event: [],
                sequent_backend_election_event_aggregate: {aggregate: {count: 0}},
            },
        }))
        const errors: string[] = []
        page.on("pageerror", (error) => errors.push(error.message))
        await page.clock.install({time: Date.parse(FIXED_TIME)})
        const unroute = await routePortal(context, portal)
        // React-admin's known telemetry image is answered locally; it never leaves the browser.
        await context.route(
            "https://react-admin-telemetry.marmelab.com/react-admin-telemetry?domain=127.0.0.1",
            (route) => {
                const request = route.request()
                if (request.method() !== "GET" || request.resourceType() !== "image") {
                    violations.add(
                        `Unexpected telemetry request: ${request.method()} ${request.resourceType()}`
                    )
                    return route.abort("blockedbyclient")
                }
                return route.fulfill({status: 204, body: ""})
            }
        )
        try {
            await use(portal)
        } finally {
            await unroute()
            expect(violations.list(), "unexpected service requests").toEqual([])
            expect(errors, "unhandled page exceptions").toEqual([])
            for (const call of graphql.calls.filter(
                (call) => call.operationName !== "GetTenantById"
            )) {
                const token = call.headers.authorization?.replace(/^Bearer /, "")
                expect(
                    token && oidc.verifyAccessToken(token),
                    `${call.operationName} must use an issued bearer token`
                ).toBeTruthy()
            }
        }
    },
})
export {expect}
