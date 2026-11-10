/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, waitFor} from "@testing-library/react"
import {downloadUrl} from "@sequentech/ui-core"
import {PerformDownload} from "./ExportElectionMenu"

const mockNotify = jest.fn()
let mockQueryResult: {loading: boolean; error?: unknown; data?: unknown}

jest.mock("@apollo/client", () => ({useQuery: () => mockQueryResult}))
jest.mock("@/queries/FetchDocument", () => ({FETCH_DOCUMENT: "FetchDocument"}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-admin", () => ({useNotify: () => mockNotify}))
jest.mock("@sequentech/ui-core", () => ({downloadUrl: jest.fn(() => Promise.resolve())}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({theme: {palette: {primary: {dark: "#000"}, white: "#fff"}}}),
    {virtual: true}
)
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {}}),
}))
jest.mock("../MiruExport", () => ({MiruExport: () => null}))
jest.mock("@/components/StyledAppAtom", () => ({StyledAppAtom: () => null}))
jest.mock("./GenerateReport", () => ({GenerateReport: () => null}))
jest.mock("./GeneratePdf", () => ({GeneratePDF: () => null}))
jest.mock("./GenerateResultsXlsx", () => ({GenerateResultsXlsx: () => null}))
jest.mock("./ExportMenuItem", () => ({ExportMenuItem: () => null}))
jest.mock("@/resources/Reports/ReportSigning", () => ({ReportRequestLinks: () => null}))

/** A refusal of `/fetch-document` as Apollo hands it to the component. */
const refused = (message: string) =>
    Object.assign(new Error(message), {
        graphQLErrors: [{message, extensions: {code: "InternalServerError"}}],
    })

const renderDownload = (onDownload: () => void) =>
    render(
        <PerformDownload
            onDownload={onDownload}
            fileName="report.tar.gz"
            documentId="document"
            electionEventId="event"
        />
    )

beforeEach(() => {
    jest.clearAllMocks()
})

it("explains that a tally archive waits for post-processing and ends the download", async () => {
    mockQueryResult = {
        loading: false,
        error: refused(
            "This operation requires electoral phase PostProcessing; current phase is Evaluation"
        ),
    }
    const done = jest.fn()
    renderDownload(done)
    await waitFor(() => expect(done).toHaveBeenCalledTimes(1))
    expect(mockNotify).toHaveBeenCalledTimes(1)
    expect(mockNotify).toHaveBeenCalledWith("refusals.documentWaitsForPostProcessing", {
        type: "error",
    })
    expect(downloadUrl).not.toHaveBeenCalled()
})

it("asks to try again while the phase is changing", async () => {
    mockQueryResult = {
        loading: false,
        error: refused("Election safety state is changing; retry the operation"),
    }
    const done = jest.fn()
    renderDownload(done)
    await waitFor(() => expect(done).toHaveBeenCalledTimes(1))
    expect(mockNotify).toHaveBeenCalledWith("refusals.phaseChanging", {type: "error"})
    expect(downloadUrl).not.toHaveBeenCalled()
})

it("keeps another failure as it was", async () => {
    mockQueryResult = {loading: false, error: refused("Document not found")}
    const done = jest.fn()
    renderDownload(done)
    await new Promise((resolve) => setTimeout(resolve, 20))
    expect(mockNotify).not.toHaveBeenCalled()
    expect(done).not.toHaveBeenCalled()
})

// Last: a started download stays marked as such until its menu resets it.
it("downloads a document the server serves", async () => {
    mockQueryResult = {
        loading: false,
        data: {fetchDocument: {url: "https://example.invalid/report"}},
    }
    const done = jest.fn()
    renderDownload(done)
    await waitFor(() => expect(done).toHaveBeenCalledTimes(1))
    expect(downloadUrl).toHaveBeenCalledWith("https://example.invalid/report", "report.tar.gz")
    expect(mockNotify).not.toHaveBeenCalled()
})
