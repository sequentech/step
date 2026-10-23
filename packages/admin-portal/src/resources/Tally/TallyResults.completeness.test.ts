/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import {TallyResults} from "./TallyResults"

const mockAlias = () => "Election"
const mockSelection: any = {
    electionId: "one",
    contestId: "contest",
    areaId: null,
    contest: {data: {id: "contest", election_id: "one"}},
    election: {data: {id: "one", election_event_id: "event"}},
}
const mockData = {
    sequent_backend_area: [],
    sequent_backend_election: [
        {id: "one", presentation: {}},
        {id: "two", presentation: {}},
    ],
    sequent_backend_contest: [],
    sequent_backend_results_election: [
        {election_id: "one", results_event_id: "result", documents: {pdf: "report"}},
    ],
    sequent_backend_results_contest: [],
    sequent_backend_results_election_area: [],
}
jest.mock("jotai", () => ({useAtomValue: () => mockData}))
jest.mock("@/atoms/tally-candidates", () => ({tallyQueryData: {}}))
jest.mock("react-admin", () => ({useRecordContext: () => ({presentation: {}})}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => mockAlias}))
jest.mock("@/hooks/useDefaultElectionLang", () => ({getDefaultElectionLang: () => "en"}))
jest.mock("../ElectionEvent/useKeysPermissions", () => ({
    useKeysPermissions: () => ({canExportCeremony: true}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@mui/material", () => ({Typography: ({children}: any) => children}))
jest.mock("./TallyElectionsResults", () => ({LoadingResults: () => "Loading"}))
jest.mock("./TallyResultsSectionGlobal", () => ({
    TallyResultsSectionGlobal: () =>
        require("react").createElement("span", null, "Global numeric results"),
}))
jest.mock("./TallyResultsSectionArea", () => ({
    TallyResultsSectionArea: () => "Area numeric results",
}))
jest.mock("@/components/tally/ExportElectionMenu", () => ({
    ExportElectionMenu: () => require("react").createElement("span", null, "Export"),
}))
jest.mock("./utils", () => ({convertContestsArray: (rows: unknown) => rows}))
jest.mock("@sequentech/ui-core", () => ({
    parseEntityPresentation: (value: unknown) => value,
    sortByPresentationOrder: (rows: unknown) => rows,
    sortContestList: (rows: unknown) => rows,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        ResultsSelectorTabs: (props: any) =>
            require("react").createElement(
                "div",
                null,
                props.renderResult(mockSelection),
                props.renderElectionActions(mockSelection),
                props.renderContestActions(mockSelection),
                props.renderAreaActions(mockSelection)
            ),
    }),
    {virtual: true}
)

const props = {
    tally: {id: "tally", election_ids: ["one", "two"]} as any,
    resultsEventId: "result",
    onCreateTransmissionPackage: jest.fn(),
    completeElectionIds: [] as string[],
}

test("incomplete elections show no numeric results or export actions", () => {
    render(React.createElement(TallyResults, props))
    expect(screen.queryByText("Global numeric results")).toBeNull()
    expect(screen.queryByText("Export")).toBeNull()
    expect(screen.getByText("Loading")).toBeTruthy()
})

test("a complete election can display results while other elections remain incomplete", () => {
    const {rerender} = render(
        React.createElement(TallyResults, {...props, completeElectionIds: ["one"]})
    )
    expect(screen.getByText("Global numeric results")).toBeTruthy()
    expect(screen.getByText("Export")).toBeTruthy()
    rerender(React.createElement(TallyResults, props))
    expect(screen.queryByText("Global numeric results")).toBeNull()
    expect(screen.queryByText("Export")).toBeNull()
})
