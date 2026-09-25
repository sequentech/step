// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect} from "@playwright/test"
import {test as base} from "@sequentech/ui-test-kit/coverage/fixture"
import {resolve} from "node:path"
import {serveDist} from "@sequentech/ui-test-kit/server/static"
import {routePortal, type PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {GraphQLMock, loadClientSchema} from "@sequentech/ui-test-kit/mocks/graphql"
import {OidcMock} from "@sequentech/ui-test-kit/mocks/oidc"
import {S3Mock} from "@sequentech/ui-test-kit/mocks/s3"
import {ViolationLog} from "@sequentech/ui-test-kit/mocks/violations"
import {electionFixture, FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"

const directory = resolve(__dirname, "../..")
export const eventPath = `/tenant/${IDS.tenant}/event/${IDS.event}`
export const realm = `tenant-${IDS.tenant}-event-${IDS.event}`
export type Portal = PortalServices & {
    data: ReturnType<typeof electionFixture>
    publish: () => void
    previewPath: string
    now: number
    castVotes: Record<string, unknown>[]
}

export const test = base.extend<{portal: Portal}, {dist: Awaited<ReturnType<typeof serveDist>>}>({
    dist: [
        // Playwright requires an object pattern even for a fixture without dependencies.
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
    portal: async ({context, page, dist}, use, testInfo) => {
        const {origin} = dist
        const violations = new ViolationLog()
        const s3 = new S3Mock({origin, violations})
        const graphql = new GraphQLMock({
            schema: loadClientSchema(resolve(directory, "graphql.schema.json")),
            violations,
        })
        const portal = {origin, violations, s3, graphql, now: Date.parse(FIXED_TIME)} as Portal
        portal.oidc = new OidcMock({
            origin,
            violations,
            now: () => portal.now,
            realms: [
                {
                    name: realm,
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
        portal.settings = {
            DISABLE_AUTH: false,
            QUERY_POLL_INTERVAL_MS: 60000,
            DEFAULT_TENANT_ID: IDS.tenant,
            DEFAULT_EVENT_ID: IDS.event,
            ONLINE_VOTING_CLIENT_ID: "voting-portal",
            KEYCLOAK_URL: portal.oidc.serverUrl(),
            HASURA_URL: `${origin}/v1/graphql`,
            PUBLIC_BUCKET_URL: s3.publicBucketUrl(),
            APP_VERSION: "test",
            APP_HASH: "test",
            BALLOT_VERIFIER_URL: `${origin}/verifier`,
            RESULTS_PORTAL_URL: `${origin}/results`,
            KEYCLOAK_ACCESS_TOKEN_LIFESPAN_SECS: 900,
            POLLING_DURATION_TIMEOUT: 12000,
        }
        portal.castVotes = []
        portal.data = electionFixture()
        portal.previewPath = `/preview/${IDS.tenant}/preview-document/${IDS.area}/publication?lang=en`
        portal.publish = () => {
            const {event, election, style, summary, ballot} = portal.data
            s3.putJson(
                "public",
                `tenant-${IDS.tenant}/event-${IDS.event}/election_event_config.json`,
                {
                    id: IDS.event,
                    tenant_id: IDS.tenant,
                    election_event_id: IDS.event,
                    election_event_presentation: event.presentation,
                }
            )
            s3.putJson(
                "public",
                `tenant-${IDS.tenant}/document-preview-document/publication.json`,
                {
                    ballot_styles: [ballot],
                    elections: [election],
                    election_event: event,
                    support_materials: [],
                    documents: [],
                }
            )
            for (const [key, document] of Object.entries({event, election, style, summary}))
                s3.putJson("private", `${key}.json`, document)
        }
        portal.publish()
        graphql.on("GetVoterStatus", (call) => {
            const bearer = call.headers.authorization?.replace(/^Bearer /, "")
            if (!bearer || !portal.oidc.verifyAccessToken(bearer))
                violations.add("GetVoterStatus without an issued bearer token")
            if (call.variables.electionEventId !== IDS.event)
                violations.add("GetVoterStatus requested another event")
            const signature = String(graphql.callsTo("GetVoterStatus").length)
            return {
                data: {
                    get_ballot_files_urls: {
                        event_id: IDS.event,
                        status: portal.data.event.status,
                        files: [
                            {
                                id: IDS.style,
                                election_id: IDS.election,
                                version: "1",
                                status: portal.data.election.status,
                                num_allowed_revotes: portal.data.election.num_allowed_revotes,
                                voting_channels: portal.data.election.voting_channels,
                                urls: {
                                    event_url: s3.presign("event.json", signature),
                                    election_url: s3.presign("election.json", signature),
                                    summary_url: s3.presign("summary.json", signature),
                                    style_url: s3.presign("style.json", signature),
                                },
                            },
                        ],
                    },
                    sequent_backend_cast_vote: portal.castVotes,
                },
            }
        })
        graphql.on("InsertCastVote", ({variables}) => ({
            data: {
                insert_cast_vote: {
                    id: "90000000-0000-4000-8000-000000000001",
                    ballot_id: variables.ballotId,
                    election_id: IDS.election,
                    election_event_id: IDS.event,
                    tenant_id: IDS.tenant,
                    area_id: IDS.area,
                    created_at: FIXED_TIME,
                    last_updated_at: FIXED_TIME,
                    labels: {},
                    annotations: {},
                    content: variables.content,
                    cast_ballot_signature: "synthetic-signature",
                    voter_id_string: IDS.voter,
                },
            },
        }))
        const errors: string[] = []
        page.on("pageerror", (error) => errors.push(error.message))
        await page.clock.install({time: portal.now})
        const unroute = await routePortal(context, portal)
        try {
            await use(portal)
        } finally {
            await unroute()
            // A known UI assertion failure must never hide a broken service boundary.
            if (violations.list().length || errors.length) testInfo.expectedStatus = "passed"
            expect(violations.list(), "unexpected service requests").toEqual([])
            expect(errors, "unhandled page exceptions").toEqual([])
        }
    },
})
export {expect, IDS, FIXED_TIME, electionFixture}
