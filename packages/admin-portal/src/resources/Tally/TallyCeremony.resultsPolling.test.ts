/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, render, screen, waitFor} from "@testing-library/react"
import {TallyCeremony} from "./TallyCeremony"

const mockCreate = jest.fn()
const mockNotify = jest.fn()
const mockRefetch = jest.fn()
const mockExecutionRefetch = jest.fn()
const mockExecutionOptions = jest.fn()
let mockSession: Record<string, unknown>
let mockSessionDataUpdatedAt = 1
let mockExecution: Record<string, unknown>
let mockTallyData: any = null
let mockResultsEvents: any[] = []
let mockCanExportCeremony = false
const mockCompleteElections = jest.fn()
const mockEventExport = jest.fn()
const mockT = (key: string) => key
const mockEmpty: unknown[] = []
const mockRecord = {
    id: "event",
    tenant_id: "tenant",
    status: {voting_status: "OPEN"},
    presentation: {},
}
let mockElections = [
    {
        id: "closed",
        keys_ceremony_id: "keys",
        status: {
            is_published: true,
            voting_status: "CLOSED",
            allow_tally: "requires-voting-period-end",
        },
    },
]
const mockKeys = {
    list_keys_ceremony: {
        items: [{id: "keys", name: "Keys", settings: {policy: "manual-ceremonies"}}],
    },
}
const mockStore = {
    tallyId: "tally",
    creatingType: "ELECTORAL_RESULTS",
    setTallyId: jest.fn(),
    setCreatingFlag: jest.fn(),
}

jest.mock("react-admin", () => ({
    useNotify: () => mockNotify,
    useRecordContext: () => mockRecord,
    useGetOne: () => ({
        data: mockSession,
        refetch: mockRefetch,
        dataUpdatedAt: mockSessionDataUpdatedAt,
    }),
    useGetList: (resource: string, _params: unknown, options: unknown) => {
        if (resource === "sequent_backend_tally_session_execution") {
            mockExecutionOptions(options)
            return {data: [mockExecution], refetch: mockExecutionRefetch}
        }
        return {
            data:
                resource === "sequent_backend_election"
                    ? mockElections
                    : resource === "sequent_backend_results_event"
                      ? mockResultsEvents
                      : mockEmpty,
            refetch: mockRefetch,
        }
    },
}))
jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useMutation: () => [mockCreate],
    useQuery: () => ({data: mockKeys}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: any) =>
            key === "keysGeneration.ceremonyStep.executionStatus" ? options.status : mockT(key),
        i18n: {language: "en", dir: () => "ltr"},
    }),
}))
jest.mock("@/providers/ElectionEventTallyProvider", () => ({
    useElectionEventTallyStore: () => mockStore,
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/WidgetsContextProvider", () => ({useWidgetStore: () => []}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true}),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_FAST_POLL_INTERVAL_MS: 1000},
    }),
}))
jest.mock("../ElectionEvent/useKeysPermissions", () => ({
    useKeysPermissions: () => ({canExportCeremony: mockCanExportCeremony}),
}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => () => "Election event"}))
jest.mock("@sequentech/ui-core", () => ({
    isArray: Array.isArray,
    isString: (value: unknown) => typeof value === "string",
    ...require("../../../../ui-core/src/types/CoreTypes"),
    ...require("../../../../ui-core/src/types/ElectionPresentation"),
    ...require("../../../../ui-core/src/types/ElectionEventPresentation"),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        BreadCrumbSteps: () => null,
        BreadCrumbStepsVariant: {Circle: "Circle"},
        theme: {
            palette: {
                info: {main: "blue"},
                warning: {main: "orange"},
                error: {main: "red"},
                background: {default: "white"},
                brandSuccess: "green",
            },
        },
        Dialog: ({
            open,
            handleClose,
        }: {
            open: boolean
            handleClose: (confirmed: boolean) => void
        }) =>
            open
                ? require("react").createElement(
                      "button",
                      {onClick: () => handleClose(true)},
                      "Confirm tally"
                  )
                : null,
    }),
    {virtual: true}
)
jest.mock("./TallyElectionsList", () => ({
    TallyElectionsList: ({
        update,
        disabled,
    }: {
        update: (ids: string[]) => void
        disabled: boolean
    }) =>
        require("react").createElement(
            "button",
            {disabled, onClick: () => update(["closed"])},
            "Select closed election"
        ),
}))
jest.mock("@/components/styles/TallyStyles", () => ({
    TallyStyles: new Proxy({}, {get: () => "div"}),
}))
jest.mock("@/components/styles/WizardStyles", () => ({
    WizardStyles: new Proxy(
        {},
        {
            get: (_target, key) =>
                key === "CeremonyStatus"
                    ? ({label}: any) =>
                          require("react").createElement(
                              "output",
                              {"data-testid": "ceremony-status"},
                              label
                          )
                    : "div",
        }
    ),
}))
jest.mock("./styles", () => ({NextButton: "button", CancelButton: "button"}))
jest.mock("@/components/ElectionHeader", () => ({__esModule: true, default: () => null}))
jest.mock("@/components/ListActions", () => ({ListActions: () => null}))
jest.mock("@/components/tally/ExportElectionMenu", () => ({
    ExportElectionMenu: (props: any) => {
        mockEventExport(props.tallySessionId)
        return require("react").createElement("button", null, "Event export")
    },
}))
jest.mock("./TallyTrusteesList", () => ({TallyTrusteesList: () => null}))
jest.mock("./TallyStartDate", () => ({TallyStartDate: () => null}))
jest.mock("./TallyElectionsProgress", () => ({TallyElectionsProgress: () => null}))
jest.mock("./TallyElectionsResults", () => ({
    TallyElectionsResults: (props: any) => {
        mockCompleteElections(props.completeElectionIds)
        return null
    },
}))
jest.mock("./TallyResults", () => ({TallyResults: () => null}))
jest.mock("./TallyLogs", () => ({TallyLogs: () => null}))
jest.mock("./TallyResolutionPanel", () => ({TallyResolutionPanel: () => null}))
jest.mock("./ResultsWebsitePublication", () => ({
    ResultsWebsitePublication: () =>
        require("react").createElement("button", null, "Publish results"),
}))
jest.mock("./ResultsDataLoader", () => ({
    ResultsDataLoader: ({
        resultsEventId,
        databaseName,
    }: {
        resultsEventId: string
        databaseName: string
    }) =>
        require("react").createElement(
            "output",
            {"data-testid": "loaded-results"},
            `${resultsEventId}:${databaseName}`
        ),
}))
jest.mock("jotai", () => ({useAtomValue: () => mockTallyData}))
jest.mock("@/atoms/tally-candidates", () => ({tallyQueryData: {}}))

const pollingInterval = () => mockExecutionOptions.mock.calls.at(-1)[0].refetchInterval

beforeEach(() => {
    jest.clearAllMocks()
    mockSessionDataUpdatedAt = 1
    mockRefetch.mockReset()
    mockRefetch.mockResolvedValue({})
    mockTallyData = null
    mockResultsEvents = []
    mockCanExportCeremony = false
    mockStore.tallyId = "tally"
    mockSession = {
        id: "tally",
        election_ids: ["closed"],
        execution_status: "IN_PROGRESS",
        is_execution_completed: false,
        annotations: {is_post_task_completed: false},
    }
    mockExecution = {
        id: "partial",
        results_event_id: "partial-results",
        documents: {sqlite: "partial-db"},
    }
    mockExecutionRefetch.mockResolvedValue({data: {data: [mockExecution], total: 1}})
})

it("refreshes the execution when completion arrives ahead of the final result snapshot", async () => {
    const {rerender} = render(React.createElement(TallyCeremony))
    mockSession = {...mockSession, execution_status: "SUCCESS", is_execution_completed: true}
    rerender(React.createElement(TallyCeremony))
    await waitFor(() => expect(mockExecutionRefetch).toHaveBeenCalled())
    expect(pollingInterval()).toBe(1000)
})

it("waits for the post-report execution refresh before stopping polling", async () => {
    let finish: (value: unknown) => void = () => undefined
    mockExecutionRefetch.mockReturnValue(
        new Promise((resolve) => {
            finish = resolve
        })
    )
    mockSession = {
        ...mockSession,
        execution_status: "SUCCESS",
        is_execution_completed: true,
        annotations: {is_post_task_completed: true},
    }
    const {rerender} = render(React.createElement(TallyCeremony))
    expect(pollingInterval()).toBe(1000)
    mockExecution = {
        id: "reported",
        results_event_id: "final-results",
        documents: {sqlite: "reported-db"},
    }
    await act(async () => {
        finish({data: {data: [mockExecution], total: 1}})
    })
    rerender(React.createElement(TallyCeremony))
    await waitFor(() => expect(pollingInterval()).toBeUndefined())
})

it("continues polling after a failed completion refresh", async () => {
    mockExecutionRefetch.mockRejectedValue(new Error("network unavailable"))
    mockSession = {
        ...mockSession,
        execution_status: "SUCCESS",
        is_execution_completed: true,
        annotations: {is_post_task_completed: true},
    }
    render(React.createElement(TallyCeremony))
    await act(async () => undefined)
    expect(pollingInterval()).toBe(1000)
})

it("resumes polling when another tally is selected and ignores the old refresh", async () => {
    let finish: (value: unknown) => void = () => undefined
    mockExecutionRefetch.mockReturnValue(
        new Promise((resolve) => {
            finish = resolve
        })
    )
    mockSession = {
        ...mockSession,
        execution_status: "SUCCESS",
        is_execution_completed: true,
        annotations: {is_post_task_completed: true},
    }
    const {rerender} = render(React.createElement(TallyCeremony))
    mockStore.tallyId = "next-tally"
    mockSession = {
        id: "next-tally",
        election_ids: ["closed"],
        execution_status: "IN_PROGRESS",
        is_execution_completed: false,
        annotations: {is_post_task_completed: false},
    }
    rerender(React.createElement(TallyCeremony))
    await act(async () => {
        finish({data: {data: [mockExecution], total: 1}})
    })
    expect(pollingInterval()).toBe(1000)
})

it("keeps the final snapshot selected when report generation replaces its SQLite document", async () => {
    mockExecution = {
        id: "counted",
        results_event_id: "final-results",
        documents: {sqlite: "final-db"},
    }
    mockExecutionRefetch.mockResolvedValue({data: {data: [mockExecution], total: 1}})
    mockSession = {...mockSession, execution_status: "SUCCESS", is_execution_completed: true}
    const {rerender} = render(React.createElement(TallyCeremony))
    await act(async () => undefined)
    expect(screen.getByTestId("loaded-results").textContent).toBe("final-results:final-db")
    expect(pollingInterval()).toBe(1000)

    mockSession = {...mockSession, annotations: {is_post_task_completed: true}}
    mockExecution = {
        id: "reported",
        results_event_id: "final-results",
        documents: {sqlite: "reported-db"},
    }
    mockExecutionRefetch.mockResolvedValue({data: {data: [mockExecution], total: 1}})
    rerender(React.createElement(TallyCeremony))
    await waitFor(() => expect(pollingInterval()).toBeUndefined())
    expect(screen.getByTestId("loaded-results").textContent).toBe("final-results:reported-db")
    rerender(React.createElement(TallyCeremony))
    expect(screen.getByTestId("loaded-results").textContent).toBe("final-results:reported-db")
})

it("resumes polling after a completed tally when selecting a new tally", async () => {
    mockSession = {
        ...mockSession,
        execution_status: "SUCCESS",
        is_execution_completed: true,
        annotations: {is_post_task_completed: true},
    }
    const {rerender} = render(React.createElement(TallyCeremony))
    await waitFor(() => expect(pollingInterval()).toBeUndefined())
    mockStore.tallyId = "next-tally"
    mockSession = {
        ...mockSession,
        id: "next-tally",
        execution_status: "IN_PROGRESS",
        is_execution_completed: false,
    }
    rerender(React.createElement(TallyCeremony))
    expect(pollingInterval()).toBe(1000)
})

it("does not show success or permit final results from an incomplete snapshot", async () => {
    mockSession = {...mockSession, execution_status: "SUCCESS", is_execution_completed: true}
    mockExecution = {
        ...mockExecution,
        tally_session_id: "tally",
        status: {elections_status: [{election_id: "closed", status: "DECRYPTING", progress: 90}]},
    }
    render(React.createElement(TallyCeremony))
    await waitFor(() =>
        expect(screen.getByTestId("ceremony-status").textContent).toBe("IN_PROGRESS")
    )
    expect(mockCompleteElections).toHaveBeenLastCalledWith([])
})

it("permits complete numeric results while withholding ceremony success until PDFs finish", async () => {
    setCompleteNumericSnapshot()
    const {rerender} = render(React.createElement(TallyCeremony))
    await waitFor(() =>
        expect(screen.getByTestId("ceremony-status").textContent).toBe("IN_PROGRESS")
    )
    expect(mockCompleteElections).toHaveBeenLastCalledWith(["closed"])
    expect(pollingInterval()).toBe(1000)
    mockSession = {...mockSession, annotations: {is_post_task_completed: true}}
    rerender(React.createElement(TallyCeremony))
    await waitFor(() => expect(screen.getByTestId("ceremony-status").textContent).toBe("SUCCESS"))
})

function setCompleteNumericSnapshot() {
    mockSession = {...mockSession, execution_status: "SUCCESS", is_execution_completed: true}
    mockExecution = {
        ...mockExecution,
        tally_session_id: "tally",
        status: {elections_status: [{election_id: "closed", status: "SUCCESS", progress: 100}]},
    }
    mockTallyData = {
        sqliteDocumentId: "partial-db",
        sequent_backend_area: [{id: "area", parent_id: null}],
        sequent_backend_results_event: [{id: "partial-results"}],
        sequent_backend_election: [{id: "closed"}],
        sequent_backend_contest: [{id: "contest", election_id: "closed"}],
        sequent_backend_area_contest: [{contest_id: "contest", area_id: "area"}],
        sequent_backend_results_election: [
            {election_id: "closed", results_event_id: "partial-results"},
        ],
        sequent_backend_results_contest: [
            {election_id: "closed", contest_id: "contest", results_event_id: "partial-results"},
        ],
        sequent_backend_results_area_contest: [
            {
                election_id: "closed",
                contest_id: "contest",
                area_id: "area",
                results_event_id: "partial-results",
            },
        ],
    }
}

it.each(["execution", "results"])(
    "recovers from a transient %s refresh failure after an unchanged successful session poll",
    async (failure) => {
        setCompleteNumericSnapshot()
        mockSession = {...mockSession, annotations: {is_post_task_completed: true}}
        mockExecutionRefetch.mockResolvedValue({data: {data: [mockExecution], total: 1}})
        if (failure === "execution")
            mockExecutionRefetch.mockRejectedValueOnce(new Error("temporary execution failure"))
        else mockRefetch.mockRejectedValueOnce(new Error("temporary result failure"))
        const {rerender} = render(React.createElement(TallyCeremony))
        await act(async () => undefined)
        expect(pollingInterval()).toBe(1000)
        expect(screen.getByTestId("ceremony-status").textContent).toBe("IN_PROGRESS")
        const attempts = mockExecutionRefetch.mock.calls.length
        // React Query structural sharing preserves the same data and IDs on a
        // successful poll; only its successful-delivery timestamp changes.
        mockSessionDataUpdatedAt += 1
        rerender(React.createElement(TallyCeremony))
        await waitFor(() => expect(pollingInterval()).toBeUndefined())
        expect(screen.getByTestId("ceremony-status").textContent).toBe("SUCCESS")
        expect(mockExecutionRefetch).toHaveBeenCalledTimes(attempts + 1)
        // Later successful deliveries must not restart finalization after success.
        mockSessionDataUpdatedAt += 1
        rerender(React.createElement(TallyCeremony))
        await act(async () => undefined)
        expect(mockExecutionRefetch).toHaveBeenCalledTimes(attempts + 1)
    }
)

it("does not restart an in-flight completion refresh on identical session polls", async () => {
    setCompleteNumericSnapshot()
    mockSession = {...mockSession, annotations: {is_post_task_completed: true}}
    let finish!: (value: unknown) => void
    mockExecutionRefetch.mockReturnValue(
        new Promise((resolve) => {
            finish = resolve
        })
    )
    const {rerender} = render(React.createElement(TallyCeremony))
    mockSessionDataUpdatedAt += 1
    rerender(React.createElement(TallyCeremony))
    expect(mockExecutionRefetch).toHaveBeenCalledTimes(1)
    await act(async () => {
        finish({data: {data: [mockExecution], total: 1}})
    })
    await waitFor(() => expect(pollingInterval()).toBeUndefined())
    expect(screen.getByTestId("ceremony-status").textContent).toBe("SUCCESS")
})

it("withholds results and ceremony success until an inherited child contest result arrives", async () => {
    setCompleteNumericSnapshot()
    mockSession = {
        ...mockSession,
        area_ids: ["area", "child"],
        annotations: {is_post_task_completed: true},
    }
    mockTallyData.sequent_backend_area.push({id: "child", parent_id: "area"})
    const {rerender} = render(React.createElement(TallyCeremony))
    await act(async () => undefined)
    expect(mockCompleteElections).toHaveBeenLastCalledWith([])
    expect(screen.getByTestId("ceremony-status").textContent).toBe("IN_PROGRESS")
    mockTallyData = {
        ...mockTallyData,
        sequent_backend_results_area_contest: [
            ...mockTallyData.sequent_backend_results_area_contest,
            {
                election_id: "closed",
                contest_id: "contest",
                area_id: "child",
                results_event_id: "partial-results",
            },
        ],
    }
    rerender(React.createElement(TallyCeremony))
    await waitFor(() => expect(screen.getByTestId("ceremony-status").textContent).toBe("SUCCESS"))
    expect(mockCompleteElections).toHaveBeenLastCalledWith(["closed"])
})

it.each(["missing area result", "stale document", "other incomplete election"])(
    "hides event-wide exports and publication for %s despite backend SUCCESS",
    async (condition) => {
        setCompleteNumericSnapshot()
        mockSession = {...mockSession, annotations: {is_post_task_completed: true}}
        mockCanExportCeremony = true
        mockResultsEvents = [
            {id: "partial-results", election_event_id: "event", documents: {json: "numbers"}},
        ]
        if (condition === "missing area result")
            mockTallyData.sequent_backend_results_area_contest = []
        if (condition === "stale document") mockTallyData.sqliteDocumentId = "older-document"
        if (condition === "other incomplete election")
            mockSession.election_ids = ["closed", "other"]
        render(React.createElement(TallyCeremony))
        await act(async () => undefined)
        expect(screen.queryByText("Event export")).toBeNull()
        expect(screen.queryByText("Publish results")).toBeNull()
        expect(screen.getByTestId("ceremony-status").textContent).toBe("IN_PROGRESS")
    }
)

it("enables event-wide exports only for the complete selected snapshot", async () => {
    setCompleteNumericSnapshot()
    mockSession = {...mockSession, annotations: {is_post_task_completed: true}}
    mockCanExportCeremony = true
    mockResultsEvents = [
        {id: "partial-results", election_event_id: "event", documents: {json: "numbers"}},
    ]
    const {rerender} = render(React.createElement(TallyCeremony))
    await waitFor(() => expect(screen.getByText("Event export")).toBeTruthy())
    expect(screen.getByText("Publish results")).toBeTruthy()
    mockTallyData = {...mockTallyData, sqliteDocumentId: "older-document"}
    rerender(React.createElement(TallyCeremony))
    expect(screen.queryByText("Event export")).toBeNull()
    expect(screen.queryByText("Publish results")).toBeNull()
})

it("does not offer a historical partial PDF link as an event-wide final export", async () => {
    setCompleteNumericSnapshot()
    mockSession = {...mockSession, annotations: {is_post_task_completed: true}}
    mockCanExportCeremony = true
    mockResultsEvents = [
        {
            id: "historical-partial-result",
            election_event_id: "event",
            documents: {pdf: "partial-pdf", html: "partial-html"},
        },
    ]
    render(React.createElement(TallyCeremony))
    await act(async () => undefined)
    expect(screen.queryByText("Event export")).toBeNull()
    expect(mockEventExport).not.toHaveBeenCalled()
})

it("never associates current PDF exports with the previous selected tally", async () => {
    setCompleteNumericSnapshot()
    mockSession = {...mockSession, annotations: {is_post_task_completed: true}}
    mockCanExportCeremony = true
    mockResultsEvents = [
        {id: "partial-results", election_event_id: "event", documents: {pdf: "report"}},
    ]
    const {rerender} = render(React.createElement(TallyCeremony))
    await waitFor(() => expect(mockEventExport).toHaveBeenCalledWith("tally"))
    mockEventExport.mockClear()
    mockStore.tallyId = "next-tally"
    mockSession = {...mockSession, id: "next-tally"}
    mockExecution = {
        ...mockExecution,
        tally_session_id: "next-tally",
        results_event_id: "next-results",
        documents: {sqlite: "next-db"},
    }
    mockExecutionRefetch.mockResolvedValue({data: {data: [mockExecution], total: 1}})
    mockResultsEvents = [
        {id: "next-results", election_event_id: "event", documents: {pdf: "next-report"}},
    ]
    mockTallyData = JSON.parse(
        JSON.stringify(mockTallyData)
            .replaceAll("partial-results", "next-results")
            .replaceAll("partial-db", "next-db")
    )
    rerender(React.createElement(TallyCeremony))
    await waitFor(() => expect(mockEventExport).toHaveBeenCalledWith("next-tally"))
    expect(mockEventExport.mock.calls.every(([id]) => id === "next-tally")).toBe(true)
})
