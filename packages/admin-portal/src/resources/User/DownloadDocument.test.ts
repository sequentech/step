/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, render, screen, waitFor, fireEvent} from "@testing-library/react"
import {
    ApolloClient,
    ApolloLink,
    ApolloProvider,
    InMemoryCache,
    NormalizedCacheObject,
    Observable,
} from "@apollo/client"
import {downloadUrl} from "@sequentech/ui-core"
import {GetDocumentQuery} from "@/gql/graphql"
import {DownloadDocument} from "./DownloadDocument"
import {GET_DOCUMENT} from "@/queries/GetDocument"

jest.mock("@sequentech/ui-core", () => ({downloadUrl: jest.fn(() => Promise.resolve())}))
let mockTenantId = "tenant"
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => [mockTenantId]}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_FAST_POLL_INTERVAL_MS: 60000},
    }),
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true}),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
const mockNotify = jest.fn()
jest.mock("react-admin", () => ({useNotify: () => mockNotify}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Dialog: ({children, handleClose}: React.PropsWithChildren<{handleClose: () => void}>) =>
            require("react").createElement(
                "div",
                {},
                children,
                require("react").createElement("button", {onClick: handleClose}, "Close")
            ),
    }),
    {virtual: true}
)
jest.mock("@/components/election-event/export-data/PasswordDialog", () => ({
    PasswordDialog: ({
        password,
        onClose,
        children,
    }: React.PropsWithChildren<{password: string; onClose: () => void}>) =>
        require("react").createElement(
            "div",
            {},
            password,
            children,
            require("react").createElement("button", {onClick: onClose}, "Close")
        ),
    DecryptHelp: ({decryptionCommand}: {decryptionCommand: string}) =>
        require("react").createElement("span", {}, decryptionCommand),
}))

let document: GetDocumentQuery["sequent_backend_document"][number] | undefined
let requestedOperations: string[]
// When set, the server refuses the download with this message.
let fetchDocumentRefusal: string | undefined
let client: ApolloClient<NormalizedCacheObject>

beforeEach(() => {
    jest.clearAllMocks()
    mockTenantId = "tenant"
    requestedOperations = []
    fetchDocumentRefusal = undefined
    document = {name: "report.epdf", annotations: {access: {password_secret_id: "vault-id"}}}
    client = new ApolloClient({
        cache: new InMemoryCache({addTypename: false}),
        link: new ApolloLink(
            (operation) =>
                new Observable((observer) => {
                    requestedOperations.push(operation.operationName)
                    if (operation.operationName === "FetchDocument" && fetchDocumentRefusal) {
                        observer.next({
                            errors: [
                                {
                                    message: fetchDocumentRefusal,
                                    extensions: {code: "InternalServerError"},
                                },
                            ],
                        } as never)
                        observer.complete()
                        return
                    }
                    const responses: Record<string, object> = {
                        GetDocument: {sequent_backend_document: document ? [document] : []},
                        FetchDocument: {
                            fetchDocument: {url: "https://example.invalid/signed-report"},
                        },
                        GetDocumentPassword: {
                            get_document_password: {password: "test-only-report-password"},
                        },
                    }
                    const data = responses[operation.operationName]
                    if (!data) {
                        observer.error(
                            new Error(`Unexpected operation: ${operation.operationName}`)
                        )
                        return
                    }
                    observer.next({data})
                    observer.complete()
                })
        ),
    })
})

afterEach(() => client.stop())

const downloadElement = (onDownload: () => void, documentId = "document") =>
    React.createElement(ApolloProvider, {
        client,
        children: React.createElement(DownloadDocument, {
            documentId,
            fileName: "old-name",
            showReportPasswordDialog: true,
            onDownload,
        }),
    })

const renderDownload = (onDownload: () => void) => render(downloadElement(onDownload))

it("retains the real password dialog after completing the download skips the metadata query", async () => {
    const done = jest.fn()
    renderDownload(done)
    await screen.findByText("test-only-report-password")
    expect(downloadUrl).toHaveBeenCalledWith("https://example.invalid/signed-report", "report.epdf")
    expect(requestedOperations.filter((name) => name === "GetDocumentPassword")).toHaveLength(1)
    expect(done).not.toHaveBeenCalled()
    fireEvent.click(screen.getByText("Close"))
    expect(done).toHaveBeenCalledTimes(1)
    expect(screen.queryByText("test-only-report-password")).toBeNull()
})

it("does not show a password dialog for an unencrypted report", async () => {
    document = {name: "report.pdf", annotations: {}}
    const done = jest.fn()
    renderDownload(done)
    await waitFor(() => expect(done).toHaveBeenCalledTimes(1))
    expect(requestedOperations).not.toContain("GetDocumentPassword")
    expect(screen.queryByText("Close")).toBeNull()
})

it.each(["document", "tenant"])(
    "clears the retained password display when the %s changes",
    async (scope) => {
        const done = jest.fn()
        const view = renderDownload(done)
        await screen.findByText("test-only-report-password")
        if (scope === "tenant") mockTenantId = "other-tenant"
        view.rerender(downloadElement(done, scope === "document" ? "other-document" : "document"))
        expect(screen.queryByText("test-only-report-password")).toBeNull()
    }
)

it("waits for committed metadata and preserves instructions for a legacy encrypted report", async () => {
    document = undefined
    const done = jest.fn()
    renderDownload(done)
    await waitFor(() => expect(requestedOperations).toContain("FetchDocument"))
    expect(downloadUrl).not.toHaveBeenCalled()
    document = {name: "legacy.epdf", annotations: {}}
    await act(async () => {
        await client.refetchQueries({include: [GET_DOCUMENT]})
    })
    await screen.findByText(/openssl enc/)
    expect(downloadUrl).toHaveBeenCalledWith("https://example.invalid/signed-report", "legacy.epdf")
    expect(requestedOperations).not.toContain("GetDocumentPassword")
    expect(done).not.toHaveBeenCalled()
    fireEvent.click(screen.getByText("Close"))
    expect(done).toHaveBeenCalledTimes(1)
})

it("explains that a document with decoded ballots waits for post-processing and ends the download", async () => {
    fetchDocumentRefusal =
        "This operation requires electoral phase PostProcessing; current phase is Evaluation"
    const done = jest.fn()
    renderDownload(done)
    await waitFor(() => expect(done).toHaveBeenCalledTimes(1))
    expect(mockNotify).toHaveBeenCalledTimes(1)
    expect(mockNotify).toHaveBeenCalledWith("refusals.documentWaitsForPostProcessing", {
        type: "error",
    })
    expect(downloadUrl).not.toHaveBeenCalled()
})

it("keeps waiting while the phase is changing, for the next poll to ask again", async () => {
    fetchDocumentRefusal = "Election safety state is changing; retry the operation"
    const done = jest.fn()
    renderDownload(done)
    await waitFor(() => expect(requestedOperations).toContain("FetchDocument"))
    await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 50))
    })
    expect(mockNotify).not.toHaveBeenCalled()
    expect(done).not.toHaveBeenCalled()
    expect(downloadUrl).not.toHaveBeenCalled()
})
