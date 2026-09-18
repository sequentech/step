/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, waitFor} from "@testing-library/react"
import {ApolloClient, ApolloLink, ApolloProvider, InMemoryCache, Observable} from "@apollo/client"
import ElectionDashboard from "./election/Dashboard"
import EventDashboard from "./election-event/Dashboard"

jest.mock("react-admin", () => ({
    useRecordContext: () => ({id: "election", election_event_id: "event"}),
    useGetList: () => ({data: []}),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {
            QUERY_POLL_INTERVAL_MS: 60000,
            VOTING_PORTAL_URL: "http://example.test",
        },
    }),
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => false}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@sequentech/ui-core", () => ({
    translateFromPresentation: () => undefined,
    VOTING_STATUS_CHANNELS: [],
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({BreadCrumbSteps: () => null, BreadCrumbStepsVariant: {Circle: "Circle"}}),
    {virtual: true}
)
jest.mock("./charts/Charts", () => ({
    getToday: () => new Date("2026-09-14"),
    daysBefore: () => new Date("2026-09-08"),
    formatDate: (date: Date) => date.toISOString(),
}))
jest.mock("./charts/VotesPerDay", () => ({VotesPerDay: () => null}))
jest.mock("./charts/VotersByChannel", () => ({VotersByChannel: () => null}))
jest.mock("@/resources/ElectionEvent/ListIpAddress", () => ({ListIpAddress: () => null}))
jest.mock("@/services/UrlGeneration", () => ({getAuthUrl: () => "http://example.test"}))
jest.mock("@/services/KeyCeremony", () => ({IKeysCeremonyExecutionStatus: {SUCCESS: "SUCCESS"}}))
jest.mock("./election/Stats", () => ({
    Stats: ({metrics}: {metrics: {eligibleVotersCount: number}}) =>
        require("react").createElement(
            "div",
            {"data-testid": "eligible"},
            metrics.eligibleVotersCount
        ),
}))
jest.mock("./election-event/Stats", () => ({
    Stats: ({metrics}: {metrics: {eligibleVotersCount: number}}) =>
        require("react").createElement(
            "div",
            {"data-testid": "eligible"},
            metrics.eligibleVotersCount
        ),
}))

it.each(["election", "event"])(
    "refreshes %s statistics on return despite a complete cached result",
    async (kind) => {
        let enabledVoters = 2
        let statsRequests = 0
        const client = new ApolloClient({
            cache: new InMemoryCache(),
            link: new ApolloLink(
                (operation) =>
                    new Observable((observer) => {
                        if (operation.operationName === "ListKeysCeremony") {
                            observer.next({
                                data: {
                                    list_keys_ceremony: {items: [], total: {aggregate: {count: 0}}},
                                },
                            })
                        } else {
                            statsRequests++
                            observer.next({
                                data: {
                                    stats: {
                                        total_eligible_voters: enabledVoters,
                                        total_distinct_voters: 0,
                                        total_areas: 1,
                                        total_elections: 1,
                                        voters_by_channel: [],
                                        votes_per_day: [],
                                    },
                                    users: {count: enabledVoters},
                                    election: [{statistics: {}}],
                                    election_event: [{statistics: {}}],
                                },
                            })
                        }
                        observer.complete()
                    })
            ),
        })
        const showDashboard = () =>
            render(
                React.createElement(ApolloProvider, {
                    client,
                    children:
                        kind === "election"
                            ? React.createElement(ElectionDashboard)
                            : React.createElement(EventDashboard, {refreshRef: React.createRef()}),
                })
            )
        try {
            const first = showDashboard()
            await waitFor(() => expect(screen.getByTestId("eligible").textContent).toBe("2"))
            first.unmount()
            enabledVoters = 1
            const second = showDashboard()
            await waitFor(() => expect(screen.getByTestId("eligible").textContent).toBe("1"))
            expect(statsRequests).toBe(2)
            second.unmount()
            enabledVoters = 2
            showDashboard()
            await waitFor(() => expect(screen.getByTestId("eligible").textContent).toBe("2"))
            expect(statsRequests).toBe(3)
        } finally {
            client.stop()
        }
    }
)
