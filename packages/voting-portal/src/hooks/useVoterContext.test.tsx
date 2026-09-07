// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, renderHook, waitFor} from "@testing-library/react"
import {ApolloClient, ApolloLink, InMemoryCache, Observable} from "@apollo/client"
import {ApolloProvider} from "@apollo/client/react"
import {MemoryRouter, Route, Routes, useNavigate} from "react-router-dom"
import {buildClientSchema, validate} from "graphql"
import schema from "../../graphql.schema.json"
import {GET_VOTER_CONTEXT} from "../queries/GetVoterContext"
import {GET_ELECTIONS} from "../queries/GetElections"
import {useVoterContext} from "./useVoterContext"

jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {DISABLE_AUTH: false}}),
}))

function response(count: number, eventId = "event") {
    return {
        sequent_backend_election_event: [
            {id: eventId, presentation: {}, status: {}, description: null},
        ],
        sequent_backend_cast_vote: [],
        sequent_backend_ballot_style: Array.from({length: count}, (_, index) => ({
            id: `style-${index}`,
            election_id: `election-${index}`,
            election_event_id: eventId,
            tenant_id: "tenant",
            area_id: "area",
            status: null,
            ballot_eml: "{}",
            ballot_signature: null,
            created_at: null,
            annotations: null,
            labels: null,
            last_updated_at: null,
            deleted_at: null,
            election: {
                id: `election-${index}`,
                tenant_id: "tenant",
                election_event_id: eventId,
                annotations: null,
                created_at: null,
                description: null,
                is_consolidated_ballot_encoding: false,
                labels: null,
                last_updated_at: null,
                num_allowed_revotes: 1,
                presentation: {},
                spoil_ballot_option: false,
                status: {voting_status: "OPEN"},
                voting_channels: {online: true},
            },
        })),
    }
}

function setup(count: number) {
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

test("the bootstrap validates against the schema with the scoped election relationship", () => {
    const extended = buildClientSchema(schema as any)
    expect(validate(extended, GET_VOTER_CONTEXT)).toEqual([])
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
        expect(operations).toEqual(["GetVoterContext"])
        expect(variables).toEqual([{tenantId: "tenant", electionEventId: "event"}])
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
    expect(operations).toEqual(["GetVoterContext"])
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
    expect(operations).toEqual(["GetVoterContext", "GetVoterContext"])
    expect(variables[1]).toEqual({tenantId: "tenant", electionEventId: "another-event"})
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
    expect(reauthenticated.operations).toEqual(["GetVoterContext"])
    next.unmount()
    reauthenticated.client.stop()
})
