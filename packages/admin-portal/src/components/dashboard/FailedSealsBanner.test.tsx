/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import {FailedSealsBanner} from "./FailedSealsBanner"
import {AuthContext} from "@/providers/AuthContextProvider"

let mockFailed: unknown[] = []
const mockUseQuery = jest.fn()

jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useQuery: (query: unknown, options: {skip?: boolean}) => {
        mockUseQuery(query, options)
        return {data: options.skip ? undefined : {sequent_backend_ballot_box_seal: mockFailed}}
    },
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
    }),
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({hasRole: () => false}),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {QUERY_POLL_INTERVAL_MS: 1}}),
}))

const withRoles = (
    roles: string[],
    presentation: unknown = {ballot_box_seal_policy: "seal-at-close"}
) =>
    render(
        <AuthContext.Provider value={{hasRole: (role: string) => roles.includes(role)} as never}>
            <FailedSealsBanner electionEventId="event" presentation={presentation} />
        </AuthContext.Provider>
    )

beforeEach(() => {
    mockFailed = []
    mockUseQuery.mockClear()
})

it("lists each failed ballot box with its election, area and reason", () => {
    mockFailed = [
        {
            id: "s1",
            election_id: "e1",
            area_id: "a1",
            area: {id: "a1", name: "Spain"},
            election_name: "Madrid Post",
            failure_reason: "a ballot does not match its Ballot ID (stored x)",
        },
    ]
    withRoles(["tally-read"])
    expect(screen.getByRole("alert")).toBeTruthy()
    expect(
        screen.getByText(
            'dashboard.ballotBoxes.incident.line {"election":"Madrid Post","area":"Spain","reason":"dashboard.ballotBoxes.failure.ballotIdMismatch"}'
        )
    ).toBeTruthy()
    expect(mockUseQuery.mock.calls[0][1]).toMatchObject({
        context: {headers: {"x-hasura-role": "tally-read"}},
    })
})

it("shows nothing when no seal failed", () => {
    withRoles(["election-dashboard-tab"])
    expect(screen.queryByRole("alert")).toBeNull()
})

it("asks nothing when the viewer has no role that reads seals", () => {
    mockFailed = [{id: "s1", election_id: "e1", area_id: "a1", failure_reason: "x"}]
    withRoles(["election-event-read"])
    expect(mockUseQuery.mock.calls[0][1]).toMatchObject({skip: true})
    expect(screen.queryByRole("alert")).toBeNull()
})

it("asks nothing for an event that doesn't seal at close", () => {
    mockFailed = [{id: "s1", election_id: "e1", area_id: "a1", failure_reason: "x"}]
    withRoles(["tally-read"], {})
    expect(mockUseQuery.mock.calls[0][1]).toMatchObject({skip: true})
    expect(screen.queryByRole("alert")).toBeNull()
})
