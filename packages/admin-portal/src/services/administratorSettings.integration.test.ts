// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
// Opt-in read-only checks against hasura/tests/administrator-settings.integration.cjs's
// disposable fixture. Every request uses a signed normal administrator bearer token.
import {ApolloClient, ApolloLink, HttpLink, InMemoryCache} from "@apollo/client"
import {createHmac} from "node:crypto"
import {administratorSettingsQuery} from "./administratorSettings"
import {createHasuraDataProvider} from "./createHasuraDataProvider"
import {FETCH_ELECTION_EVENTS_TREE} from "../queries/GetElectionEventsTree"

const origin = process.env.SETTINGS_TEST_URL
const live = origin && process.env.SETTINGS_TEST_JWT_KEY ? describe : describe.skip
live("actual administrator portal documents against Hasura", () => {
    const tenant = "11111111-1111-4111-8111-111111111111"
    const event = "33333333-3333-4333-8333-333333333333"
    let client: ApolloClient<any>
    beforeAll(() => {
        expect(new URL(origin!).hostname).toBe("127.0.0.1")
        const encode = (value: unknown) => Buffer.from(JSON.stringify(value)).toString("base64url")
        const body =
            encode({alg: "HS256", typ: "JWT"}) +
            "." +
            encode({
                "sub": "fixture-human",
                "iat": Math.floor(Date.now() / 1000),
                "exp": Math.floor(Date.now() / 1000) + 300,
                "https://hasura.io/jwt/claims": {
                    "x-hasura-default-role": "admin-user",
                    "x-hasura-allowed-roles": ["admin-user"],
                    "x-hasura-user-id": "44444444-4444-4444-8444-444444444444",
                    "x-hasura-tenant-id": tenant,
                },
            })
        const token =
            body +
            "." +
            createHmac("sha256", process.env.SETTINGS_TEST_JWT_KEY!)
                .update(body)
                .digest("base64url")
        client = new ApolloClient({
            cache: new InMemoryCache(),
            link: new ApolloLink((operation, forward) => {
                operation.query = administratorSettingsQuery(operation.query, operation.variables)
                return forward(operation)
            }).concat(
                new HttpLink({
                    uri: origin + "/v1/graphql",
                    fetch,
                    headers: {"authorization": "Bearer " + token, "x-hasura-role": "admin-user"},
                })
            ),
        })
    })
    afterAll(() => client?.stop())
    it("opens the real ElectionEventList getList shape and settings detail", async () => {
        const provider = await createHasuraDataProvider(client)
        const result = await provider.getList("sequent_backend_election_event", {
            pagination: {page: 1, perPage: 10},
            sort: {field: "created_at", order: "DESC"},
            filter: {tenant_id: tenant, is_archived: false},
        })
        expect(result.total).toBe(1)
        expect(result.data[0].id).toBe(event)
        const detail = await provider.getOne("sequent_backend_election_event", {id: event})
        expect(detail.data.id).toBe(event)
        expect(detail.data.presentation.i18n.en.name).toBe("Context")
    })
    it("loads the actual event navigation document with projected names", async () => {
        const result = await client.query({
            query: FETCH_ELECTION_EVENTS_TREE,
            variables: {tenantId: tenant, isArchived: false},
            fetchPolicy: "network-only",
        })
        expect(result.data.sequent_backend_election_event[0].presentation.i18n.en.name).toBe(
            "Context"
        )
        expect(JSON.stringify(result.data)).not.toContain("ballot data")
    })
})
