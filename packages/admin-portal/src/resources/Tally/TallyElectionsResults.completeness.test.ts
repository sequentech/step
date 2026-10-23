/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import {TallyElectionsResults} from "./TallyElectionsResults"

let mockData: any
const mockAlias = () => "Election"
const mockPolicy = {}
const mockRecord = {presentation: {}}
jest.mock("jotai", () => ({useAtomValue: () => mockData}))
jest.mock("@/atoms/tally-candidates", () => ({tallyQueryData: {}}))
jest.mock("react-admin", () => ({useRecordContext: () => mockRecord}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => mockAlias}))
jest.mock("@/hooks/useDefaultElectionLang", () => ({getDefaultElectionLang: () => "en"}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {}}),
}))
jest.mock("@/components/NoItem", () => ({NoItem: () => "Empty"}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@mui/material", () => ({Box: ({children}: any) => children}))
jest.mock("@mui/x-data-grid", () => ({
    DataGrid: ({rows}: any) =>
        require("react").createElement(
            "output",
            {"data-testid": "totals"},
            JSON.stringify(rows.map((row: any) => [row.id, row.total_voters]))
        ),
}))
jest.mock("react-apexcharts", () => ({__esModule: true, default: () => null}))
jest.mock("@/components/dashboard/charts/Charts", () => ({
    __esModule: true,
    default: ({children}: any) => children,
}))
jest.mock("@sequentech/ui-core", () => ({
    EBlankBallotsPolicy: {ENABLED: "enabled"},
    EDeclineToVotePolicy: {ENABLED: "enabled"},
    isNumber: (value: unknown) => typeof value === "number",
    parseEntityPresentation: (value: unknown) => value,
    sortByPresentationOrder: (rows: unknown) => rows,
    useNumberFormat: () => ({formatNumber: String, policy: mockPolicy}),
    formatPercentOne: String,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({Loader: () => "Loading", withEscapedChartText: (value: unknown) => value}),
    {virtual: true}
)
jest.mock("./utils", () => ({
    orderItemsByIds: (rows: any[], ids: string[]) => rows.filter((row) => ids.includes(row.id)),
}))

const props = {
    tenantId: "tenant",
    electionEventId: "event",
    resultsEventId: "A",
    electionIds: ["one", "two"],
    isMultiContest: false,
    completeElectionIds: ["one"],
}

beforeEach(() => {
    mockData = {
        sequent_backend_election: [
            {id: "one", presentation: {}},
            {id: "two", presentation: {}},
        ],
        sequent_backend_results_event: [{id: "A"}, {id: "B"}],
        sequent_backend_results_election: [
            {election_id: "one", results_event_id: "A", total_voters: 12, elegible_census: 20},
            {election_id: "one", results_event_id: "B", total_voters: 18, elegible_census: 20},
            {election_id: "two", results_event_id: "A", total_voters: 3, elegible_census: 20},
        ],
        sequent_backend_results_contest: [],
    }
})

test("only complete elections have numeric totals", () => {
    render(React.createElement(TallyElectionsResults, props))
    expect(screen.getByTestId("totals").textContent).toBe('[["one",12]]')
})

test("switching result identity recomputes totals even when query arrays are unchanged", () => {
    const {rerender} = render(React.createElement(TallyElectionsResults, props))
    rerender(React.createElement(TallyElectionsResults, {...props, resultsEventId: "B"}))
    expect(screen.getByTestId("totals").textContent).toBe('[["one",18]]')
})

test.each(["loading", "empty", "incomplete"])(
    "%s cannot retain an earlier final-looking total",
    (state) => {
        const {rerender} = render(React.createElement(TallyElectionsResults, props))
        if (state === "loading") mockData = null
        if (state === "empty") mockData = {...mockData, sequent_backend_results_election: []}
        rerender(React.createElement(TallyElectionsResults, {...props, completeElectionIds: []}))
        expect(screen.queryByTestId("totals")).toBeNull()
    }
)
