// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect} from "@playwright/test"
import {buildSchema} from "graphql"
import {mkdtemp, rm, writeFile} from "node:fs/promises"
import {tmpdir} from "node:os"
import {join} from "node:path"
import {createServer} from "node:http"
import type {AddressInfo} from "node:net"
import {GraphQLMock} from "../mocks/graphql"
import {OidcMock, decodeJwt} from "../mocks/oidc"
import {S3Mock} from "../mocks/s3"
import {ViolationLog} from "../mocks/violations"
import type {MockRequest, MockFulfillment} from "../mocks/http"
import {serveDist} from "../server/static"
import {routePortal} from "../adapters/playwright"

const origin = "http://127.0.0.1:12345"
const request = (path: string, method = "GET", body?: string): MockRequest => ({
    url: new URL(path, origin),
    method,
    headers: {},
    body,
})
const bodyOf = (response: MockFulfillment) =>
    JSON.parse(String(response.body)) as Record<string, unknown>

function oidcFixture() {
    const violations = new ViolationLog()
    let now = Date.parse("2026-01-01T00:00:00Z")
    const oidc = new OidcMock({
        origin,
        violations,
        now: () => now,
        realms: ["election", "other"].map((name) => ({
            name,
            clients: ["voting-portal"],
            user: {id: "synthetic-user", username: "voter"},
        })),
    })
    // RFC 7636 appendix B's published S256 example is independent of our hash helper.
    const verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
    const params = new URLSearchParams({
        client_id: "voting-portal",
        redirect_uri: `${origin}/callback`,
        response_type: "code",
        scope: "openid",
        state: "state123",
        nonce: "nonce123",
        code_challenge_method: "S256",
        code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
    })
    const authorize = () => {
        const response = oidc.handle(
            request(`/keycloak/realms/election/protocol/openid-connect/auth?${params}`)
        ) as MockFulfillment
        expect(response.status).toBe(302)
        const callback = new URL(response.headers!.location)
        const values = new URLSearchParams(callback.hash.slice(1))
        expect(values.get("state")).toBe("state123")
        return new URLSearchParams({
            client_id: "voting-portal",
            grant_type: "authorization_code",
            redirect_uri: `${origin}/callback`,
            code: values.get("code")!,
            code_verifier: verifier,
        })
    }
    const token = (form: URLSearchParams, realm = "election") =>
        oidc.handle(
            request(
                `/keycloak/realms/${realm}/protocol/openid-connect/token`,
                "POST",
                form.toString()
            )
        ) as MockFulfillment
    return {
        oidc,
        violations,
        authorize,
        token,
        advance: (milliseconds: number) => {
            now += milliseconds
        },
    }
}

test("OIDC keeps state and nonce, verifies RFC PKCE, refreshes and expires tokens", () => {
    const fixture = oidcFixture()
    const response = fixture.token(fixture.authorize())
    expect(response.status).toBe(200)
    const tokens = bodyOf(response)
    expect(decodeJwt(String(tokens.id_token))).toMatchObject({
        nonce: "nonce123",
        sub: "synthetic-user",
        aud: "voting-portal",
    })
    expect(fixture.oidc.verifyAccessToken(String(tokens.access_token))).toBeDefined()
    const refresh = new URLSearchParams({
        client_id: "voting-portal",
        grant_type: "refresh_token",
        refresh_token: String(tokens.refresh_token),
    })
    expect(fixture.token(refresh).status).toBe(200)
    fixture.advance(900000)
    expect(fixture.oidc.verifyAccessToken(String(tokens.access_token))).toBeUndefined()
    expect(fixture.violations.list()).toEqual([])
})

for (const [field, invalid] of [
    ["code_verifier", "incorrect-verifier"],
    ["redirect_uri", `${origin}/elsewhere`],
    ["client_id", "other-client"],
]) {
    test(`OIDC rejects ${field} changed after a valid authorization`, () => {
        const fixture = oidcFixture()
        expect(fixture.token(fixture.authorize()).status).toBe(200)
        const form = fixture.authorize()
        form.set(field, invalid)
        expect(fixture.token(form).status).toBe(400)
        expect(fixture.violations.list()).toHaveLength(1)
    })
}

test("OIDC authorization codes cannot be replayed", () => {
    const fixture = oidcFixture()
    const form = fixture.authorize()
    expect(fixture.token(form).status).toBe(200)
    expect(fixture.token(form).status).toBe(400)
    expect(fixture.violations.list()[0]).toContain("already used")
})

test("OIDC refresh token cannot cross realms", () => {
    const fixture = oidcFixture()
    const tokens = bodyOf(fixture.token(fixture.authorize()))
    const form = new URLSearchParams({
        client_id: "voting-portal",
        grant_type: "refresh_token",
        refresh_token: String(tokens.refresh_token),
    })
    expect(fixture.token(form).status).toBe(200)
    expect(fixture.token(form, "other").status).toBe(400)
    expect(fixture.violations.list()[0]).toContain("realm")
})

function graphqlFixture() {
    const violations = new ViolationLog()
    const graphql = new GraphQLMock({
        schema: buildSchema("type Query { greeting(name: String!): String! }"),
        violations,
    })
    graphql.on("Greeting", ({variables}) => ({
        data: {greeting: `Hello ${variables.name}`, ignored: "not selected"},
    }))
    const send = (
        query = "query Greeting($name: String!) { greeting(name: $name) }",
        variables: unknown = {name: "Voter"}
    ) =>
        graphql.handle(
            request(
                "/v1/graphql",
                "POST",
                JSON.stringify({operationName: "Greeting", query, variables})
            )
        )
    return {graphql, violations, send}
}

test("GraphQL validates variables, executes the selection and records headers", async () => {
    const fixture = graphqlFixture()
    expect(bodyOf((await fixture.send()) as MockFulfillment)).toEqual({
        data: {greeting: "Hello Voter"},
    })
    expect(fixture.graphql.callsTo("Greeting")[0].variables).toEqual({name: "Voter"})
    expect(fixture.violations.list()).toEqual([])
})

test("GraphQL infers a sole operation and rejects ambiguous or unknown names", async () => {
    const fixture = graphqlFixture()
    const query = "query Greeting($name: String!) { greeting(name: $name) }"
    const send = (body: object) =>
        fixture.graphql.handle(
            request("/v1/graphql", "POST", JSON.stringify({variables: {name: "Voter"}, ...body}))
        )
    expect(bodyOf((await send({query})) as MockFulfillment)).toEqual({
        data: {greeting: "Hello Voter"},
    })
    expect(fixture.graphql.callsTo("Greeting")).toHaveLength(1)
    fixture.graphql.on("", () => ({data: {greeting: "Hello Anonymous"}}))
    expect(
        bodyOf((await send({query: '{ greeting(name: "Anonymous") }'})) as MockFulfillment)
    ).toEqual({data: {greeting: "Hello Anonymous"}})
    expect(fixture.violations.list()).toEqual([])
    expect(
        bodyOf(
            (await send({
                query: `${query} query Other { greeting(name: "Other") }`,
            })) as MockFulfillment
        )
    ).toHaveProperty("errors")
    expect(
        bodyOf((await send({query, operationName: "Missing"})) as MockFulfillment)
    ).toHaveProperty("errors")
    expect(fixture.graphql.calls).toHaveLength(2)
    expect(fixture.violations.list()).toHaveLength(2)
})

for (const [name, query, variables] of [
    ["unknown field", "query Greeting { unknown }", {}],
    ["missing required variable", "query Greeting($name: String!) { greeting(name: $name) }", {}],
    ["wrong variable type", "query Greeting($name: String!) { greeting(name: $name) }", {name: 42}],
] as const) {
    test(`GraphQL rejects ${name} after a valid request`, async () => {
        const fixture = graphqlFixture()
        expect(bodyOf((await fixture.send()) as MockFulfillment)).toEqual({
            data: {greeting: "Hello Voter"},
        })
        expect(bodyOf((await fixture.send(query, variables)) as MockFulfillment)).toHaveProperty(
            "errors"
        )
        expect(fixture.graphql.calls).toHaveLength(1)
        expect(fixture.violations.list()).toHaveLength(1)
    })
}

test("GraphQL response data must satisfy the schema and queued errors are consumed once", async () => {
    const fixture = graphqlFixture()
    fixture.graphql.once("Greeting", () => ({
        errors: [{message: "temporary", extensions: {code: "Unavailable"}}],
    }))
    expect(bodyOf((await fixture.send()) as MockFulfillment)).toEqual({
        errors: [{message: "temporary", extensions: {code: "Unavailable"}}],
    })
    expect(bodyOf((await fixture.send()) as MockFulfillment)).toEqual({
        data: {greeting: "Hello Voter"},
    })
    fixture.graphql.on("Greeting", () => ({data: {greeting: null}}))
    expect(bodyOf((await fixture.send()) as MockFulfillment)).toHaveProperty("errors")
    expect(fixture.violations.list()).toHaveLength(1)
})

test("S3 delivers exact bytes, HEAD metadata, expiring URL overrides and detects missing objects", () => {
    const violations = new ViolationLog()
    const s3 = new S3Mock({origin, violations})
    s3.putBytes("private", "ballot.json", new Uint8Array([0, 1, 255]), "application/octet-stream")
    const url = s3.presign("ballot.json", "expired")
    const req = request(url)
    expect(s3.handle(req)).toMatchObject({status: 200, body: new Uint8Array([0, 1, 255])})
    expect(s3.handle({...req, method: "HEAD"})).toMatchObject({status: 200, body: undefined})
    s3.override(({query}) => query["X-Amz-Signature"] === "expired", {status: 403}, 1)
    expect(s3.handle(req)).toMatchObject({status: 403})
    expect(s3.handle(req)).toMatchObject({status: 200})
    expect(violations.list()).toEqual([])
    s3.delete("private", "ballot.json")
    expect(s3.handle(req)).toMatchObject({status: 404})
    expect(violations.list()).toHaveLength(1)
})

test("production server owns an ephemeral port and never returns HTML for missing assets", async () => {
    const directory = await mkdtemp(join(tmpdir(), "ui-kit-"))
    await writeFile(join(directory, "index.html"), "<main>Portal</main>")
    await writeFile(join(directory, "asset.js"), "export const value = 7")
    const server = await serveDist(directory)
    try {
        expect(
            await (
                await fetch(`${server.origin}/tenant/event`, {headers: {accept: "text/html"}})
            ).text()
        ).toBe("<main>Portal</main>")
        expect(await (await fetch(`${server.origin}/asset.js`)).text()).toBe(
            "export const value = 7"
        )
        expect((await fetch(`${server.origin}/missing.js`)).status).toBe(404)
        expect((await fetch(`${server.origin}/asset.js`, {method: "POST"})).status).toBe(405)
    } finally {
        await server.close()
        await rm(directory, {recursive: true, force: true})
    }
})

test("unexpected WebSockets are reported without reaching a server", async ({context, page}) => {
    let upgrades = 0
    const server = createServer((_request, response) => response.end("<main>Portal</main>"))
    server.on("upgrade", (_request, socket) => {
        upgrades += 1
        socket.destroy()
    })
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve))
    const address = `http://127.0.0.1:${(server.address() as AddressInfo).port}`
    const socketUrl = address.replace("http:", "ws:") + "/unexpected"
    const violations = new ViolationLog()
    const unroute = await routePortal(context, {
        origin: address,
        settings: {},
        graphql: new GraphQLMock({schema: buildSchema("type Query { ok: Boolean }"), violations}),
        oidc: new OidcMock({origin: address, violations, realms: []}),
        s3: new S3Mock({origin: address, violations}),
        violations,
    })
    try {
        await page.goto(address)
        await page.evaluate(
            (url) =>
                new Promise<void>((resolve) => {
                    const socket = new WebSocket(url)
                    socket.onclose = () => resolve()
                }),
            socketUrl
        )
        expect(violations.list()).toEqual([`Unexpected WebSocket: ${socketUrl}`])
        expect(upgrades).toBe(0)
    } finally {
        await unroute()
        await new Promise<void>((resolve, reject) => {
            server.close((error) => (error ? reject(error) : resolve()))
            server.closeAllConnections()
        })
    }
})
