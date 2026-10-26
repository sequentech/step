/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, waitFor} from "@testing-library/react"
import {ResultsDataLoader} from "./ResultsDataLoader"

const mockSet = jest.fn()
const mockQuery = jest.fn()
let mockAuthorized = true
const mockRows: Record<string, Array<Record<string, unknown>>> = {}
const mockEmpty: never[] = []
jest.mock("jotai", () => ({atom: () => ({}), useSetAtom: () => mockSet}))
jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useQuery: (...args: unknown[]) => mockQuery(...args),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => mockAuthorized}),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_POLL_INTERVAL_MS: 1000},
    }),
}))
jest.mock("@/hooks/useSQLiteDatabase", () => ({
    useManagedDatabase: () => ({isReady: true, error: null}),
    useSQLQuery: (sql: string) => ({
        data: mockRows[sql.match(/FROM (\w+)/)![1]] ?? mockEmpty,
        isReady: true,
    }),
}))
jest.mock("@sequentech/ui-core", () => ({isString: (value: unknown) => typeof value === "string"}))
const post = {
    id: "post-row",
    tenant_id: "tenant",
    election_event_id: "event",
    results_event_id: "results",
    election_id: "post",
    total_voters: 0,
    documents: {json: "json", pdf: "old-pdf"},
}
const area = {...post, id: "area-row", area_id: "country"}
const props = {
    resultsEventId: "results",
    electionEventId: "event",
    isTallyCompleted: true,
    contests: [],
    electionIds: ["post"],
    databaseName: "snapshot",
}
const output = () => mockSet.mock.calls.filter(([data]) => data !== null).at(-1)![0]
beforeEach(() => {
    jest.clearAllMocks()
    mockAuthorized = true
    mockRows.results_election = [post]
    mockRows.results_election_area = [area]
    mockQuery.mockReturnValue({
        data: {
            sequent_backend_results_election: [
                {...post, documents: {json: "json", pdf: "signed-pdf"}},
            ],
            sequent_backend_results_election_area: [
                {...area, documents: {json: "area-json", pdf: "signed-area"}},
            ],
        },
    })
})
test("updates released Post and country PDFs without changing immutable tally counts", async () => {
    const view = render(<ResultsDataLoader {...props} />)
    await waitFor(() =>
        expect(output().sequent_backend_results_election[0].documents.pdf).toBe("signed-pdf")
    )
    expect(output().sequent_backend_results_election[0].total_voters).toBe(0)
    expect(output().sequent_backend_results_election_area[0].documents.pdf).toBe("signed-area")
    mockQuery.mockReturnValue({
        data: {
            sequent_backend_results_election: [
                {...post, total_voters: 999, documents: {pdf: "replacement-pdf"}},
            ],
            sequent_backend_results_election_area: [],
        },
    })
    view.rerender(<ResultsDataLoader {...props} />)
    await waitFor(() =>
        expect(output().sequent_backend_results_election[0].documents.pdf).toBe("replacement-pdf")
    )
    expect(output().sequent_backend_results_election[0].total_voters).toBe(0)
    expect(post.documents.pdf).toBe("old-pdf")
})
test("rejects another results session or country despite matching row ID", async () => {
    mockQuery.mockReturnValue({
        data: {
            sequent_backend_results_election: [{...post, results_event_id: "other"}],
            sequent_backend_results_election_area: [{...area, area_id: "other"}],
        },
    })
    render(<ResultsDataLoader {...props} />)
    await waitFor(() => expect(output().sequent_backend_results_election[0].documents).toEqual({}))
    expect(output().sequent_backend_results_election_area[0].documents).toEqual({})
})
test("does not reuse stale document IDs after a denied current read", async () => {
    mockQuery.mockReturnValue({
        error: new Error("denied"),
        data: {sequent_backend_results_election: [post]},
    })
    render(<ResultsDataLoader {...props} />)
    await waitFor(() => expect(output().sequent_backend_results_election[0].documents).toEqual({}))
})
test("uses held tally-results-read with exact scope and polling", async () => {
    render(<ResultsDataLoader {...props} />)
    await waitFor(() => expect(mockQuery).toHaveBeenCalled())
    expect(mockQuery.mock.calls[0][1]).toMatchObject({
        variables: {
            tenantId: "tenant",
            electionEventId: "event",
            resultsEventId: "results",
            electionIds: ["post"],
        },
        context: {headers: {"x-hasura-role": "tally-results-read"}},
        fetchPolicy: "network-only",
        pollInterval: 1000,
        skip: false,
    })
})
test("does not request a role the viewer lacks", async () => {
    mockAuthorized = false
    render(<ResultsDataLoader {...props} />)
    await waitFor(() => expect(mockQuery).toHaveBeenCalled())
    expect(mockQuery.mock.calls[0][1].skip).toBe(true)
    expect(output().sequent_backend_results_election[0].documents).toEqual({})
})
