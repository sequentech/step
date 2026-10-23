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
let mockExecution: Record<string, unknown>
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
    useGetOne: () => ({data: mockSession, refetch: mockRefetch}),
    useGetList: (resource: string, _params: unknown, options: unknown) => {
        if (resource === "sequent_backend_tally_session_execution") {
            mockExecutionOptions(options)
            return {data: [mockExecution], refetch: mockExecutionRefetch}
        }
        return {
            data: resource === "sequent_backend_election" ? mockElections : mockEmpty,
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
    useTranslation: () => ({t: mockT, i18n: {language: "en", dir: () => "ltr"}}),
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
jest.mock("../ElectionEvent/useKeysPermissions", () => ({useKeysPermissions: () => ({})}))
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
    WizardStyles: new Proxy({}, {get: () => "div"}),
}))
jest.mock("./styles", () => ({NextButton: "button", CancelButton: "button"}))
jest.mock("@/components/ElectionHeader", () => ({__esModule: true, default: () => null}))
jest.mock("@/components/ListActions", () => ({ListActions: () => null}))
jest.mock("@/components/tally/ExportElectionMenu", () => ({ExportElectionMenu: () => null}))
jest.mock("./TallyTrusteesList", () => ({TallyTrusteesList: () => null}))
jest.mock("./TallyStartDate", () => ({TallyStartDate: () => null}))
jest.mock("./TallyElectionsProgress", () => ({TallyElectionsProgress: () => null}))
jest.mock("./TallyElectionsResults", () => ({TallyElectionsResults: () => null}))
jest.mock("./TallyResults", () => ({TallyResults: () => null}))
jest.mock("./TallyLogs", () => ({TallyLogs: () => null}))
jest.mock("./TallyResolutionPanel", () => ({TallyResolutionPanel: () => null}))
jest.mock("./ResultsWebsitePublication", () => ({ResultsWebsitePublication: () => null}))
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
jest.mock("@/atoms/tally-candidates", () => ({tallyQueryData: {}}))

const pollingInterval = () => mockExecutionOptions.mock.calls.at(-1)[0].refetchInterval

beforeEach(() => {
    jest.clearAllMocks()
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
    mockExecutionRefetch.mockResolvedValue({data: [mockExecution]})
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
        finish({data: [mockExecution]})
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
        finish({data: [mockExecution]})
    })
    expect(pollingInterval()).toBe(1000)
})

it("keeps the final snapshot selected when report generation replaces its SQLite document", async () => {
    mockExecution = {
        id: "counted",
        results_event_id: "final-results",
        documents: {sqlite: "final-db"},
    }
    mockExecutionRefetch.mockResolvedValue({data: [mockExecution]})
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
    mockExecutionRefetch.mockResolvedValue({data: [mockExecution]})
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
