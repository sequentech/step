// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test as base, expect} from "@playwright/test"
import {resolve} from "node:path"
import {serveDist} from "@sequentech/ui-test-kit/server/static"
import {routePortal, type PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {GraphQLMock, loadClientSchema} from "@sequentech/ui-test-kit/mocks/graphql"
import {OidcMock} from "@sequentech/ui-test-kit/mocks/oidc"
import {S3Mock} from "@sequentech/ui-test-kit/mocks/s3"
import {ViolationLog} from "@sequentech/ui-test-kit/mocks/violations"
import {FIXED_TIME, IDS, electionFixture} from "@sequentech/ui-test-kit/fixtures"

const directory = resolve(__dirname, "../..")
export const eventPath = `/tenant/${IDS.tenant}/event/${IDS.event}/start?lang=en`
export const realm = `tenant-${IDS.tenant}-event-${IDS.event}`

export function verifierServices(origin: string): PortalServices {
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
                name: realm,
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
    graphql.on("GetBallotStyles", ({headers}) => {
        const token = headers.authorization?.replace(/^Bearer /, "")
        if (!token || !oidc.verifyAccessToken(token))
            violations.add("Verifier did not send an issued bearer token")
        return {data: {sequent_backend_ballot_publication: [], sequent_backend_ballot_style: []}}
    })
    return {
        origin,
        violations,
        s3,
        graphql,
        oidc,
        settings: {
            DISABLE_AUTH: false,
            QUERY_POLL_INTERVAL_MS: 60000,
            DEFAULT_TENANT_ID: IDS.tenant,
            DEFAULT_EVENT_ID: IDS.event,
            ONLINE_VOTING_CLIENT_ID: "ballot-verifier",
            KEYCLOAK_URL: oidc.serverUrl(),
            HASURA_URL: `${origin}/v1/graphql`,
            PUBLIC_BUCKET_URL: s3.publicBucketUrl(),
            APP_VERSION: "test",
            APP_HASH: "test",
        },
    }
}
export const test = base.extend<
    {portal: PortalServices},
    {dist: Awaited<ReturnType<typeof serveDist>>}
>({
    dist: [
        // Playwright requires destructuring even without fixture dependencies.
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
        const portal = verifierServices(dist.origin)
        const errors: string[] = []
        page.on("pageerror", (error) => errors.push(error.message))
        await page.clock.install({time: Date.parse(FIXED_TIME)})
        const unroute = await routePortal(context, portal)
        try {
            await use(portal)
        } finally {
            await unroute()
            // An expected accessibility failure must never excuse a broken service boundary.
            if (portal.violations.list().length || errors.length)
                test.info().expectedStatus = "passed"
            expect(portal.violations.list(), "unexpected service requests").toEqual([])
            expect(errors, "unhandled page exceptions").toEqual([])
        }
    },
})
export {expect, IDS}
