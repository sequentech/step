// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, renderHook, waitFor} from "@testing-library/react"
import {ApolloClient, ApolloLink, InMemoryCache, Observable} from "@apollo/client"
import {ApolloProvider} from "@apollo/client/react"
import {MemoryRouter, Route, Routes, useNavigate} from "react-router-dom"
import {buildClientSchema, validate} from "graphql"
import schema from "../../graphql.schema.json"
import {GET_VOTER_STATUS} from "../queries/GetVoterStatus"
import {GET_ELECTIONS} from "../queries/GetElections"
import {useVoterContext} from "./useVoterContext"

jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {DISABLE_AUTH: false}}),
}))

const objects = new Map<string, unknown>()
const downloads: string[] = []
beforeEach(() => {
    objects.clear()
    downloads.length = 0
    global.fetch = jest.fn(async (url) => {
        downloads.push(String(url))
        return new Response(JSON.stringify(objects.get(String(url))), {status: 200})
    })
})

function response(count: number, eventId = "event") {
    const eventUrl = `https://objects/${eventId}/event`
    objects.set(eventUrl, {id: eventId, presentation: {}, status: {}, description: null})
    return {
        sequent_backend_cast_vote: [],
        get_ballot_files_urls: {
            event_id: eventId,
            status: {},
            files: Array.from({length: count}, (_, index) => {
                const urls = {
                    event_url: eventUrl,
                    election_url: `https://objects/${eventId}/election-${index}`,
                    summary_url: `https://objects/${eventId}/summary-${index}`,
                    style_url: `https://objects/${eventId}/style-${index}`,
                }
                objects.set(urls.election_url, {
                    id: `election-${index}`,
                    tenant_id: "tenant",
                    election_event_id: eventId,
                    annotations: null,
                    created_at: null,
                    description: null,
                    is_consolidated_ballot_encoding: false,
                    labels: null,
                    last_updated_at: null,
                    presentation: {},
                    spoil_ballot_option: false,
                })
                objects.set(urls.summary_url, {
                    id: `style-${index}`,
                    area_presentation: {},
                    election_dates: {},
                })
                objects.set(urls.style_url, {
                    id: `style-${index}`,
                    election_id: `election-${index}`,
                    election_event_id: eventId,
                    tenant_id: "tenant",
                    ballot_eml: "{}",
                })
                return {
                    id: `style-${index}`,
                    election_id: `election-${index}`,
                    version: "v1",
                    urls,
                    status: {voting_status: "OPEN"},
                    num_allowed_revotes: 1,
                    voting_channels: {online: true},
                }
            }),
        },
    }
}

function setup(count: number, networkError?: () => Error | undefined) {
    const operations: string[] = []
    const variables: unknown[] = []
    const client = new ApolloClient({
        cache: new InMemoryCache(),
        link: new ApolloLink(
            (operation) =>
                new Observable((observer) => {
                    operations.push(operation.operationName || "")
                    variables.push(operation.variables)
                    const timer = setTimeout(() => {
                        const error = networkError?.()
                        if (error) {
                            observer.error(error)
                            return
                        }
                        observer.next({data: response(count, operation.variables.electionEventId)})
                        observer.complete()
                    }, 10)
                    return () => clearTimeout(timer)
                })
        ),
    })
    const wrapper = ({children}: {children: React.ReactNode}) => (
        <ApolloProvider client={client}>
            <MemoryRouter initialEntries={["/tenant/tenant/event/event"]}>
                <Routes>
                    <Route path="/tenant/:tenantId/event/:eventId" element={children} />
                </Routes>
            </MemoryRouter>
        </ApolloProvider>
    )
    return {client, operations, variables, wrapper}
}

test("the minimal status request validates against the schema", () => {
    const extended = buildClientSchema(schema as any)
    expect(validate(extended, GET_VOTER_STATUS)).toEqual([])
})

test.each([0, 1, 200])(
    "%i eligible elections use one request, including delayed eligibility",
    async (count) => {
        const {client, operations, variables, wrapper} = setup(count)
        const {result, unmount} = renderHook(() => useVoterContext(), {wrapper})
        expect(result.current.loading).toBe(true)
        expect(result.current.elections).toBeUndefined()
        await waitFor(() =>
            expect(result.current.elections?.sequent_backend_election).toHaveLength(count)
        )
        expect(downloads.some((url) => url.includes("/style-"))).toBe(false)
        expect(operations).toEqual(["GetVoterStatus"])
        expect(variables).toEqual([{electionEventId: "event"}])
        unmount()
        client.stop()
    }
)

test("review, confirmation and a second mount reuse loaded election data", async () => {
    const {client, operations, wrapper} = setup(2)
    const first = renderHook(() => useVoterContext(), {wrapper})
    await waitFor(() =>
        expect(first.result.current.elections?.sequent_backend_election).toHaveLength(2)
    )
    await client.query({query: GET_ELECTIONS, variables: {electionIds: ["election-0"]}})
    await client.query({
        query: GET_ELECTIONS,
        variables: {electionIds: ["election-0", "election-1"]},
    })
    first.unmount()
    const second = renderHook(() => useVoterContext(), {wrapper})
    await waitFor(() => expect(second.result.current.loading).toBe(false))
    expect(operations).toEqual(["GetVoterStatus"])
    second.unmount()
    client.stop()
})

test("changing event scope waits for a new scoped response", async () => {
    const {client, operations, variables, wrapper} = setup(1)
    const {result, unmount} = renderHook(
        () => ({context: useVoterContext(), navigate: useNavigate()}),
        {wrapper}
    )
    await waitFor(() => expect(result.current.context.loading).toBe(false))
    act(() => result.current.navigate("/tenant/tenant/event/another-event"))
    expect(result.current.context.loading).toBe(true)
    expect(result.current.context.data).toBeUndefined()
    await waitFor(() =>
        expect(result.current.context.data?.sequent_backend_election_event[0].id).toBe(
            "another-event"
        )
    )
    expect(operations).toEqual(["GetVoterStatus", "GetVoterStatus"])
    expect(variables[1]).toEqual({electionEventId: "another-event"})
    unmount()
    client.stop()
})

test("a fresh authenticated client does not reuse the previous client's eligibility", async () => {
    const first = setup(1)
    const mounted = renderHook(() => useVoterContext(), {wrapper: first.wrapper})
    await waitFor(() =>
        expect(mounted.result.current.elections?.sequent_backend_election).toHaveLength(1)
    )
    mounted.unmount()
    first.client.stop()
    const reauthenticated = setup(0)
    const next = renderHook(() => useVoterContext(), {wrapper: reauthenticated.wrapper})
    expect(next.result.current.data).toBeUndefined()
    await waitFor(() => expect(next.result.current.elections?.sequent_backend_election).toEqual([]))
    expect(reauthenticated.operations).toEqual(["GetVoterStatus"])
    next.unmount()
    reauthenticated.client.stop()
})

test("direct entry downloads only the selected ballot and reuses list objects", async () => {
    const {client, wrapper} = setup(200)
    const {result, unmount} = renderHook(() => useVoterContext("election-42"), {wrapper})
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.data?.sequent_backend_ballot_style[0].id).toBe("style-42")
    expect(downloads.filter((url) => url.includes("/style-"))).toEqual([
        "https://objects/event/style-42",
    ])
    expect(downloads.filter((url) => url.endsWith("/event"))).toHaveLength(1)
    expect(downloads.filter((url) => url.includes("/election-"))).toHaveLength(1)
    expect(downloads.filter((url) => url.includes("/summary-"))).toHaveLength(1)
    unmount()
    client.stop()
})

test("expired object URLs get one authenticated renewal, with no retry loop", async () => {
    const {client, operations, wrapper} = setup(1)
    global.fetch = jest.fn(async () => new Response("expired", {status: 403}))
    const {result, unmount} = renderHook(() => useVoterContext(), {wrapper})
    await waitFor(() => expect(result.current.error).toBeDefined())
    expect(operations).toEqual(["GetVoterStatus", "GetVoterStatus"])
    expect(result.current.data).toBeUndefined()
    expect(result.current.error?.message).not.toContain("https:")
    unmount()
    client.stop()
})

test("an unlisted election cannot trigger a ballot object download", async () => {
    const {client, wrapper} = setup(1)
    const {result, unmount} = renderHook(() => useVoterContext("unauthorized"), {wrapper})
    await waitFor(() => expect(result.current.error).toBeDefined())
    expect(downloads.some((url) => url.includes("/style-"))).toBe(false)
    expect(result.current.data).toBeUndefined()
    unmount()
    client.stop()
})

test("mismatched immutable election data is rejected before caching", async () => {
    const {client, wrapper} = setup(1)
    const original = global.fetch
    global.fetch = jest.fn(async (url, init) =>
        String(url).includes("/election-")
            ? new Response(JSON.stringify({id: "another-election", election_event_id: "event"}))
            : original(url, init)
    )
    const {result, unmount} = renderHook(() => useVoterContext(), {wrapper})
    await waitFor(() => expect(result.current.error).toBeDefined())
    expect(result.current.data).toBeUndefined()
    unmount()
    client.stop()
})

test("retry restarts failed downloads with unchanged metadata and keeps successful objects", async () => {
    const {client, operations, wrapper} = setup(2)
    const original = global.fetch
    let failSummary = true
    global.fetch = jest.fn(async (url, init) =>
        failSummary && String(url).endsWith("/summary-1")
            ? new Response("temporary failure", {status: 503})
            : original(url, init)
    )
    const {result, unmount} = renderHook(() => useVoterContext(), {wrapper})
    await waitFor(() => expect(result.current.error).toBeDefined())
    expect(result.current.loading).toBe(false)
    const statusData = client.readQuery({
        query: GET_VOTER_STATUS,
        variables: {electionEventId: "event"},
    })
    await act(async () => {
        await result.current.retry()
    })
    await waitFor(() => expect(result.current.error).toBeDefined())
    expect(result.current.loading).toBe(false)
    failSummary = false
    await act(async () => {
        await result.current.retry()
    })
    await waitFor(() => expect(result.current.data?.sequent_backend_election).toHaveLength(2))
    expect(result.current.error).toBeUndefined()
    expect(result.current.loading).toBe(false)
    expect(client.readQuery({query: GET_VOTER_STATUS, variables: {electionEventId: "event"}})).toBe(
        statusData
    )
    expect(operations).toEqual(["GetVoterStatus"])
    expect(downloads.filter((url) => url.endsWith("/event"))).toHaveLength(1)
    expect(downloads.filter((url) => url.endsWith("/summary-0"))).toHaveLength(1)
    expect(downloads.some((url) => url.includes("/style-"))).toBe(false)
    unmount()
    client.stop()
})

test("retry reports repeated metadata request failures and recovers when the request succeeds", async () => {
    let failRequest = true
    const {client, operations, wrapper} = setup(1, () =>
        failRequest ? new Error("offline") : undefined
    )
    const {result, unmount} = renderHook(() => useVoterContext(), {wrapper})
    await waitFor(() => expect(result.current.error).toBeDefined())
    expect(result.current.loading).toBe(false)
    await act(async () => {
        await result.current.retry()
    })
    await waitFor(() => expect(result.current.error).toBeDefined())
    expect(result.current.loading).toBe(false)
    expect(downloads).toHaveLength(0)
    failRequest = false
    await act(async () => {
        await result.current.retry()
    })
    await waitFor(() => expect(result.current.data?.sequent_backend_election).toHaveLength(1))
    expect(result.current.error).toBeUndefined()
    expect(result.current.loading).toBe(false)
    expect(operations).toEqual(["GetVoterStatus", "GetVoterStatus", "GetVoterStatus"])
    unmount()
    client.stop()
})

test("an explicit retry gets one new expired-URL renewal and can recover", async () => {
    const {client, operations, wrapper} = setup(1)
    const original = global.fetch
    global.fetch = jest.fn(async () => new Response("expired", {status: 403}))
    const {result, unmount} = renderHook(() => useVoterContext(), {wrapper})
    await waitFor(() => expect(result.current.error).toBeDefined())
    expect(operations).toHaveLength(2)
    let expiredOnce = true
    global.fetch = jest.fn(async (url, init) => {
        if (expiredOnce) {
            expiredOnce = false
            return new Response("expired", {status: 403})
        }
        return original(url, init)
    })
    await act(async () => {
        await result.current.retry()
    })
    await waitFor(() => expect(result.current.data?.sequent_backend_election).toHaveLength(1))
    expect(result.current.error).toBeUndefined()
    expect(operations).toHaveLength(3)
    unmount()
    client.stop()
})
