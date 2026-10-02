/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import "@testing-library/jest-dom"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {
    ApolloClient,
    ApolloLink,
    ApolloProvider,
    InMemoryCache,
    NormalizedCacheObject,
    Observable,
} from "@apollo/client"
import {downloadUrl} from "@sequentech/ui-core"
import {IPermissions} from "@/types/keycloak"
import {RequestsTab} from "./RequestsTab"
import {signaturesAccess} from "./signingSettings"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../../ui-core/src/utils/typechecks"),
    downloadUrl: jest.fn(() => Promise.resolve()),
}))
jest.mock("@sequentech/ui-essentials", () => ({Dialog: () => null}), {virtual: true})
jest.mock("@/components/election-event/export-data/PasswordDialog", () => ({
    PasswordDialog: () => null,
    DecryptHelp: () => null,
}))
jest.mock("react-admin", () => ({useGetList: () => ({data: []}), useNotify: () => jest.fn()}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => () => ""}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true}),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_FAST_POLL_INTERVAL_MS: 60000},
    }),
}))
jest.mock("@/components/signing/SigningProvider", () => ({
    useSigningRequest: () => ({open: jest.fn()}),
}))

const EXPORT_URL = "https://storage.invalid/signing-requests.csv?signature=short-lived"
const DOCUMENT_URL = "https://storage.invalid/document-route"

/** What an OFOV or Auditor holds: the requests, but not the documents. */
const access = signaturesAccess((permission) =>
    [IPermissions.SIGNING_REQUESTS_READ, IPermissions.SIGNING_REQUESTS_EXPORT].includes(permission)
)

let operations: string[]
let client: ApolloClient<NormalizedCacheObject>

const clientExporting = (url: string | null) =>
    new ApolloClient({
        cache: new InMemoryCache({addTypename: false}),
        link: new ApolloLink(
            (operation) =>
                new Observable((observer) => {
                    operations.push(operation.operationName)
                    const responses: Record<string, object> = {
                        GetSigningRequests: {sequent_backend_signing_request: []},
                        SigningEventInfo: {signingEventInfo: {time_zone: null, titles: {}}},
                        SigningExportRequests: {
                            signingExportRequests: {
                                document_id: "document",
                                sha256: "a".repeat(64),
                                rows: 0,
                                url,
                            },
                        },
                        GetDocument: {sequent_backend_document: [{name: "export.csv"}]},
                        FetchDocument: {fetchDocument: {url: DOCUMENT_URL}},
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

const exportRequests = async (url: string | null) => {
    client = clientExporting(url)
    render(
        <ApolloProvider client={client}>
            <RequestsTab electionEventId="event" access={access} />
        </ApolloProvider>
    )
    fireEvent.click(await screen.findByText("signing.requests.exportCsv"))
}

beforeEach(() => {
    jest.clearAllMocks()
    operations = []
})

afterEach(() => client.stop())

it("downloads the export from its link, without the document routes", async () => {
    await exportRequests(EXPORT_URL)
    await waitFor(() =>
        expect(downloadUrl).toHaveBeenCalledWith(EXPORT_URL, "signing.requests.exportFileName")
    )
    // The export can be run again once the download started.
    await waitFor(() =>
        expect(screen.getByText("signing.requests.exportCsv").closest("button")).toBeEnabled()
    )
    // Beside the event's zone, which the times are shown in.
    expect(operations.filter((name) => name !== "SigningEventInfo")).toEqual([
        "GetSigningRequests",
        "SigningExportRequests",
    ])
})

it("downloads through the document routes when the export has no link", async () => {
    await exportRequests(null)
    await waitFor(() =>
        expect(downloadUrl).toHaveBeenCalledWith(DOCUMENT_URL, "signing.requests.exportFileName")
    )
    expect(operations).toEqual(expect.arrayContaining(["GetDocument", "FetchDocument"]))
})
