/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, renderHook} from "@testing-library/react"
import {
    CreateElectionEventProvider,
    useCreateElectionEventStore,
} from "./CreateElectionEventContextProvider"

jest.mock("uuid", () => ({v4: () => "synthetic-event-id"}))

const mockImport = jest.fn()
const mockTask = jest.fn()
const mockFailed = jest.fn()
const mockCreated = jest.fn()
const mockNavigate = jest.fn()
const mockTenant = {id: "tenant-a", settings: {}}
jest.mock("@apollo/client", () => ({
    gql: (source: TemplateStringsArray) => source.join(""),
    useApolloClient: () => ({refetchQueries: jest.fn()}),
    useMutation: (document: string) => [
        document.includes("mutation ImportElectionEvent") ? mockImport : jest.fn(),
    ],
}))
jest.mock("react-admin", () => ({
    useGetOne: () => ({data: mockTenant}),
    useGetList: () => ({data: []}),
    useNotify: () => jest.fn(),
    useRefresh: () => jest.fn(),
}))
jest.mock("react-router-dom", () => ({useNavigate: () => mockNavigate}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@sequentech/ui-core", () => ({isNull: (value: unknown) => value === null}))
// Not resolvable from the portal's jest; the real `readProblems` is tested in
// ui-essentials. This one keeps its contract: an array, or nothing.
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        readProblems: (value: unknown) =>
            Array.isArray(value) && value.length > 0 ? value : undefined,
    }),
    {virtual: true}
)
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-a"]}))
jest.mock("@/providers/WidgetsContextProvider", () => ({
    useWidgetStore: () => [() => ({identifier: "widget-a"}), mockTask, mockFailed],
}))
jest.mock("@/providers/NewResourceProvider", () => ({
    NewResourceContext: require("react").createContext({
        setLastCreatedResource: (value: unknown) => mockCreated(value),
    }),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_POLL_INTERVAL_MS: 100},
    }),
}))
jest.mock("@/services/i18n", () => ({addDefaultTranslationsToElement: jest.fn()}))

const CHECKSUM = "ab".repeat(32)
beforeEach(() => {
    jest.clearAllMocks()
    mockImport.mockReset().mockResolvedValue({
        data: {
            import_election_event: {id: "event-a", error: null, task_execution: {id: "task-a"}},
        },
    })
})
it.each(["", "synthetic archive password"])(
    "forwards an operator checksum with password %p before tracking import",
    async (password) => {
        const {result} = renderHook(useCreateElectionEventStore, {
            wrapper: CreateElectionEventProvider,
        })
        await act(async () =>
            result.current.handleImportElectionEvent("document-a", CHECKSUM, password)
        )
        expect(mockImport).toHaveBeenCalledWith({
            variables: {tenantId: "tenant-a", documentId: "document-a", sha256: CHECKSUM, password},
        })
        expect(mockTask).toHaveBeenCalledWith("widget-a", "task-a", expect.any(Function))
        expect(mockCreated).toHaveBeenCalledWith({
            id: "event-a",
            type: "sequent_backend_election_event",
        })
        expect(mockFailed).not.toHaveBeenCalled()
    }
)
it("keeps a confirmed empty checksum optional and reports a backend integrity failure", async () => {
    mockImport.mockResolvedValueOnce({
        data: {
            import_election_event: {
                id: null,
                error: "Checksum mismatch",
                task_execution: {id: "task-a"},
            },
        },
    })
    const {result} = renderHook(useCreateElectionEventStore, {wrapper: CreateElectionEventProvider})
    await act(async () => result.current.handleImportElectionEvent("document-a", "", ""))
    expect(mockImport).toHaveBeenCalledWith({
        variables: {tenantId: "tenant-a", documentId: "document-a", sha256: "", password: ""},
    })
    expect(result.current.errors).toBe("Checksum mismatch")
    // The refused task is followed, not just marked, so the widget can load the
    // problems it was failed with.
    expect(mockTask).toHaveBeenCalledWith("widget-a", "task-a")
    expect(mockFailed).not.toHaveBeenCalled()
    expect(mockCreated).not.toHaveBeenCalled()
})
it("marks the widget failed when a refusal names no task to follow", async () => {
    mockImport.mockResolvedValueOnce({
        data: {import_election_event: {id: null, error: "Checksum mismatch", task_execution: null}},
    })
    const {result} = renderHook(useCreateElectionEventStore, {wrapper: CreateElectionEventProvider})
    await act(async () => result.current.handleImportElectionEvent("document-a", "", ""))
    expect(result.current.errors).toBe("Checksum mismatch")
    expect(mockFailed).toHaveBeenCalledWith("widget-a")
    expect(mockTask).not.toHaveBeenCalled()
})
it("checks the uploaded archive before starting a task without treating validation as import", async () => {
    mockImport.mockResolvedValueOnce({data: {import_election_event: {error: null}}})
    const {result} = renderHook(useCreateElectionEventStore, {wrapper: CreateElectionEventProvider})
    await act(async () => result.current.uploadCallback("document-a", "archive password"))
    expect(mockImport).toHaveBeenCalledWith({
        variables: {
            tenantId: "tenant-a",
            documentId: "document-a",
            password: "archive password",
            checkOnly: true,
        },
    })
    expect(mockCreated).not.toHaveBeenCalled()
    expect(mockTask).not.toHaveBeenCalled()
})
it("keeps the named problems of a refused archive beside its error text", async () => {
    const problem = {
        severity: "error",
        code: "unreadable",
        path: "file",
        message: "the file is not a readable election event",
        id: "file.not-json",
    }
    mockImport.mockResolvedValueOnce({
        data: {import_election_event: {error: "Error checking import", problems: [problem]}},
    })
    const {result} = renderHook(useCreateElectionEventStore, {wrapper: CreateElectionEventProvider})
    await act(async () =>
        expect(result.current.uploadCallback("document-a", "")).rejects.toThrow(
            "Error checking import"
        )
    )
    expect(result.current.errors).toBe("Error checking import")
    expect(result.current.problems).toEqual([problem])

    // Opening the drawer again starts from nothing.
    await act(async () => result.current.openImportDrawer())
    expect(result.current.errors).toBeNull()
    expect(result.current.problems).toBeNull()
})
