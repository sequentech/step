// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect} from "@playwright/test"
import {test as base} from "@sequentech/ui-test-kit/coverage/fixture"
import {fileURLToPath} from "node:url"
import {resolve} from "node:path"
import {serveDist} from "@sequentech/ui-test-kit/server/static"
import {routePortal, type PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {GraphQLMock, loadClientSchema} from "@sequentech/ui-test-kit/mocks/graphql"
import {OidcMock} from "@sequentech/ui-test-kit/mocks/oidc"
import {S3Mock} from "@sequentech/ui-test-kit/mocks/s3"
import {ViolationLog} from "@sequentech/ui-test-kit/mocks/violations"
import {FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {resultsFixture, resultsIds} from "../fixtures/results"
import {sqliteBytes} from "../fixtures/sqlite"

const directory = fileURLToPath(new URL("../../", import.meta.url))
export const eventPath = `/${resultsIds.event}?lang=en`
export const realm = `tenant-${resultsIds.tenant}-event-${resultsIds.event}`
export type Portal = PortalServices & {
    data: ReturnType<typeof resultsFixture>
    publish: () => Promise<void>
}

export const test = base.extend<{portal: Portal}, {dist: Awaited<ReturnType<typeof serveDist>>}>({
    dist: [
        // Playwright requires a destructured fixture argument.
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
    portal: async ({context, page, dist}, use) => {
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
            realms: [resultsIds.event, "other-event"].map((eventId) => ({
                name: `tenant-${resultsIds.tenant}-event-${eventId}`,
                clients: ["results-portal"],
                user: {id: "results-voter", username: "results-voter", firstName: "Pat"},
                claims: () => ({
                    "https://hasura.io/jwt/claims": {
                        "x-hasura-default-role": "voter",
                        "x-hasura-tenant-id": resultsIds.tenant,
                        "x-hasura-election-event-id": eventId,
                        "x-hasura-area-id": "north",
                    },
                }),
            })),
        })
        const portal: Portal = {
            origin,
            violations,
            s3,
            graphql,
            oidc,
            settings: {
                RESULTS_PORTAL_CLIENT_ID: "results-portal",
                KEYCLOAK_URL: oidc.serverUrl(),
                HASURA_URL: `${origin}/v1/graphql`,
                PUBLIC_BUCKET_URL: s3.publicBucketUrl(),
                APP_VERSION: "test",
                APP_HASH: "test",
            },
            data: resultsFixture(),
            publish: async () => {
                s3.putJson("public", `results-index/${resultsIds.event}.json`, portal.data.index)
                s3.putJson("public", "results/manifest.json", portal.data.manifest)
                const bytes = await sqliteBytes(portal.data.dataset)
                s3.putBytes("public", "results/full.sqlite", bytes, "application/vnd.sqlite3")
                s3.putBytes("private", "results/area.sqlite", bytes, "application/vnd.sqlite3")
            },
        }
        const requireToken = (headers: Record<string, string>) => {
            const token = headers.authorization?.replace(/^Bearer /, "")
            if (!token || !oidc.verifyAccessToken(token))
                violations.add("Results query did not use an issued bearer token")
        }
        graphql.on("ResolveResultsPublication", ({headers}) => {
            requireToken(headers)
            return {
                data: {
                    resolveResultsPublication: {
                        tenant_id: resultsIds.tenant,
                        election_event_id: resultsIds.event,
                        access: "authenticated",
                        route_scope: "event",
                        election_ids: portal.data.manifest.election_ids,
                        publication_id: portal.data.manifest.publication_id,
                        manifest: portal.data.manifest,
                    },
                },
            }
        })
        graphql.on("FetchResultsArtifact", ({headers}) => {
            requireToken(headers)
            return {
                data: {
                    fetchResultsArtifact: {
                        urls: [s3.presign("results/area.sqlite", "results-signature")],
                    },
                },
            }
        })
        await portal.publish()
        const errors: string[] = []
        page.on("pageerror", (error) => errors.push(error.message))
        await page.clock.install({time: Date.parse(FIXED_TIME)})
        const unroute = await routePortal(context, portal)
        try {
            await use(portal)
        } finally {
            await unroute()
            // Expected accessibility failures cannot excuse unrelated browser/service failures.
            if (violations.list().length || errors.length) test.info().expectedStatus = "passed"
            expect(violations.list(), "unexpected service requests").toEqual([])
            expect(errors, "unhandled page exceptions").toEqual([])
        }
    },
})
export {expect, resultsIds}
