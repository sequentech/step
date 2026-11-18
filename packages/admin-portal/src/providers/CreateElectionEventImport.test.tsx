/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, render} from "@testing-library/react"
import {
    CreateElectionEventProvider,
    useCreateElectionEventStore,
} from "./CreateElectionEventContextProvider"
jest.mock("uuid", () => ({v4: () => "synthetic-uuid"}))
const mockMutation = jest.fn()
const mockTask = jest.fn()
const mockFail = jest.fn()
const mockNotify = jest.fn()
jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useMutation: () => [mockMutation],
    useApolloClient: () => ({refetchQueries: jest.fn()}),
}))
jest.mock("react-admin", () => ({
    useGetOne: () => ({}),
    useGetList: () => ({}),
    useNotify: () => mockNotify,
    useRefresh: () => jest.fn(),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-router-dom", () => ({useNavigate: () => jest.fn()}))
jest.mock("@sequentech/ui-core", () => ({isNull: (value: unknown) => value == null}))
jest.mock("@sequentech/ui-essentials", () => ({readProblems: (value: unknown) => value}), {
    virtual: true,
})
jest.mock("@/providers/NewResourceProvider", () => ({
    NewResourceContext: require("react").createContext({setLastCreatedResource: jest.fn()}),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_POLL_INTERVAL_MS: 3000},
    }),
}))
jest.mock("@/providers/WidgetsContextProvider", () => ({
    useWidgetStore: () => [() => ({identifier: "synthetic-widget"}), mockTask, mockFail],
}))
jest.mock("./TenantContextProvider", () => ({useTenantStore: () => ["synthetic-tenant"]}))
jest.mock("@/services/i18n", () => ({addDefaultTranslationsToElement: jest.fn()}))
jest.mock("jotai", () => ({useAtom: () => [false, jest.fn()]}))
jest.mock("@/atoms/archived-election-event-selection", () => ({__esModule: true, default: {}}))
let store: ReturnType<typeof useCreateElectionEventStore>
const Probe = () => {
    store = useCreateElectionEventStore()
    return null
}
beforeEach(() => {
    jest.clearAllMocks()
    render(
        <CreateElectionEventProvider>
            <Probe />
        </CreateElectionEventProvider>
    )
})
it("requires a real backend response to confirm checkOnly validation", async () => {
    mockMutation.mockResolvedValue({data: undefined})
    await act(async () => {
        await expect(store.uploadCallback("synthetic-document")).rejects.toMatchObject({
            stage: "transport",
        })
    })
    expect(mockMutation).toHaveBeenCalledWith({
        variables: {
            tenantId: "synthetic-tenant",
            documentId: "synthetic-document",
            password: "",
            checkOnly: true,
        },
    })
    expect(mockTask).not.toHaveBeenCalled()
})
it("retains typed backend dry-run problems and rejects invalid packages", async () => {
    const problems = [
        {
            severity: "error",
            code: "missing-reference",
            path: "Candidates row 2 contest",
            message: "Synthetic missing contest",
        },
    ]
    mockMutation.mockResolvedValue({
        data: {import_election_event: {error: "Synthetic invalid package", problems}},
    })
    await act(async () => {
        await expect(store.uploadCallback("synthetic-document")).rejects.toMatchObject({
            stage: "validation",
        })
    })
    expect(store.problems).toEqual(problems)
    expect(mockTask).not.toHaveBeenCalled()
})
it("keeps the drawer open when creation fails after a passing dry-run", async () => {
    mockMutation
        .mockResolvedValueOnce({data: {import_election_event: {error: null}}})
        .mockResolvedValueOnce({
            data: {
                import_election_event: {
                    error: "Synthetic creation failure",
                    task_execution: {id: "failed-task"},
                },
            },
        })
    act(() => store.openImportDrawer())
    await act(async () => {
        await store.uploadCallback("synthetic-document")
    })
    expect(store.importDrawer).toBe(true)
    await act(async () => {
        await expect(
            store.handleImportElectionEvent("synthetic-document", "synthetic-digest")
        ).rejects.toMatchObject({stage: "creation"})
    })
    expect(store.importDrawer).toBe(true)
    expect(mockTask).toHaveBeenCalledWith("synthetic-widget", "failed-task")
    expect(mockMutation.mock.calls[1][0].variables.checkOnly).toBeUndefined()
})
it("closes the drawer only after a backend creation task has been accepted", async () => {
    mockMutation.mockResolvedValue({
        data: {
            import_election_event: {id: "synthetic-event", task_execution: {id: "synthetic-task"}},
        },
    })
    act(() => store.openImportDrawer())
    await act(async () => {
        await store.handleImportElectionEvent("synthetic-document", "synthetic-digest")
    })
    expect(store.importDrawer).toBe(false)
    expect(mockTask).toHaveBeenCalledWith(
        "synthetic-widget",
        "synthetic-task",
        expect.any(Function)
    )
    expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.creationSubmitted", {
        type: "info",
    })
})

it("does not announce submission when the backend omits the creation task ID", async () => {
    mockMutation.mockResolvedValue({
        data: {import_election_event: {id: "synthetic-event", task_execution: null}},
    })
    act(() => store.openImportDrawer())
    await act(async () => {
        await expect(
            store.handleImportElectionEvent("synthetic-document", "synthetic-digest")
        ).rejects.toMatchObject({stage: "creation"})
    })
    expect(store.importDrawer).toBe(true)
    expect(mockNotify).not.toHaveBeenCalled()
})

// meta#14760: a different major version is a warning. The dry run keeps it
// for the import's diagnostics and passes; an error beside it still refuses;
// a later clean upload clears it.
const majorMismatch = {
    severity: "warning",
    code: "incompatible_version",
    path: "version",
    message: "the file was exported by version v10.0.0, a different major version",
    id: "file.version-major-mismatch",
}
it("keeps a dry run's warnings and passes it", async () => {
    mockMutation.mockResolvedValue({
        data: {import_election_event: {error: null, problems: [majorMismatch]}},
    })
    await act(async () => {
        await store.uploadCallback("synthetic-document")
    })
    expect(store.problems).toEqual([majorMismatch])
    expect(store.errors).toBeNull()
})
it("refuses a dry run with an error beside the version warning", async () => {
    const fatal = {
        severity: "error",
        code: "dangling_reference",
        path: "contests[0].election_id",
        message: "Synthetic dangling election",
    }
    mockMutation.mockResolvedValue({
        data: {import_election_event: {error: null, problems: [majorMismatch, fatal]}},
    })
    await act(async () => {
        await expect(store.uploadCallback("synthetic-document")).rejects.toMatchObject({
            stage: "validation",
        })
    })
    expect(store.problems).toEqual([majorMismatch, fatal])
})
it("clears a previous upload's warnings when a clean file is checked", async () => {
    mockMutation
        .mockResolvedValueOnce({
            data: {import_election_event: {error: null, problems: [majorMismatch]}},
        })
        .mockResolvedValueOnce({data: {import_election_event: {error: null}}})
    await act(async () => {
        await store.uploadCallback("first-document")
    })
    expect(store.problems).toEqual([majorMismatch])
    await act(async () => {
        await store.uploadCallback("second-document")
    })
    expect(store.problems).toBeNull()
})
