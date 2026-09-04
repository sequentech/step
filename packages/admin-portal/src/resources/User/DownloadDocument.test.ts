/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, waitFor, fireEvent} from "@testing-library/react"
import {useQuery} from "@apollo/client"
import {downloadUrl} from "@sequentech/ui-core"
import {DownloadDocument} from "./DownloadDocument"
import {GET_DOCUMENT} from "@/queries/GetDocument"

jest.mock("@apollo/client", () => ({useQuery: jest.fn(), gql: (s: any) => s.join("")}))
jest.mock("react-admin", () => ({useGetOne: jest.fn()}))
jest.mock("@sequentech/ui-core", () => ({downloadUrl: jest.fn(() => Promise.resolve())}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_FAST_POLL_INTERVAL_MS: 1000},
    }),
}))
jest.mock("../Reports/ReportPasswordDialog", () => ({
    ReportPasswordDialog: ({documentId, onClose}: any) =>
        require("react").createElement("button", {onClick: onClose}, `Password for ${documentId}`),
}))

let mockDocument: any
beforeEach(() => {
    jest.clearAllMocks()
    mockDocument = {name: "report.epdf", annotations: {access: {password_secret_id: "vault-id"}}}
    ;(useQuery as jest.Mock).mockImplementation((query) =>
        query === GET_DOCUMENT
            ? {
                  data: {sequent_backend_document: mockDocument ? [mockDocument] : []},
                  stopPolling: jest.fn(),
              }
            : {
                  data: {fetchDocument: {url: "https://example.invalid/signed-report"}},
                  refetch: jest.fn(),
              }
    )
})

it("downloads the signed URL with its extension then retains the password dialog until closed", async () => {
    const done = jest.fn()
    render(
        React.createElement(DownloadDocument, {
            documentId: "document",
            fileName: "old-name",
            showReportPasswordDialog: true,
            onDownload: done,
        })
    )
    await screen.findByText("Password for document")
    expect(downloadUrl).toHaveBeenCalledWith("https://example.invalid/signed-report", "report.epdf")
    expect(done).not.toHaveBeenCalled()
    fireEvent.click(screen.getByText("Password for document"))
    expect(done).toHaveBeenCalledTimes(1)
})

it("does not show a password dialog for an unencrypted report", async () => {
    mockDocument = {name: "report.pdf"}
    const done = jest.fn()
    render(
        React.createElement(DownloadDocument, {
            documentId: "document",
            fileName: null,
            showReportPasswordDialog: true,
            onDownload: done,
        })
    )
    await waitFor(() => expect(done).toHaveBeenCalledTimes(1))
    expect(screen.queryByText("Password for document")).toBeNull()
})

it("waits for committed metadata before starting a report download", async () => {
    mockDocument = undefined
    const props = {
        documentId: "document",
        fileName: "report",
        showReportPasswordDialog: true,
        onDownload: jest.fn(),
    }
    const view = render(React.createElement(DownloadDocument, props))
    expect(downloadUrl).not.toHaveBeenCalled()
    mockDocument = {name: "legacy.epdf"}
    view.rerender(React.createElement(DownloadDocument, props))
    await screen.findByText("Password for document")
    expect(downloadUrl).toHaveBeenCalledTimes(1)
})
