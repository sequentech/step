/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import type {DocumentNode} from "graphql"
import {getOperationName} from "@apollo/client/utilities"
import {ListTally} from "./ListTally"
import {ITallyExecutionStatus, ITallyTrusteeStatus} from "@/types/ceremonies"
import {IPermissions} from "@/types/keycloak"
import {Sequent_Backend_Tally_Session} from "@/gql/graphql"

const POLL_INTERVAL_MS = 5000
const TRUSTEE_NAME = "trustee-1"
const LIST_PAGE_SIZE = 10
const ADD_KEY = "tallysheet.common.tallyCeremony.addKey"
const VIEW = "tallysheet.common.tallyCeremony.view"
const RECORD_TALLY = {id: "tally-01"} as Sequent_Backend_Tally_Session

interface MockTallySession {
    id: string
    created_at: string
    execution_status: ITallyExecutionStatus
}

interface MockExecution {
    tally_session_id: string
    status: {trustees: Array<{name: string; status: ITallyTrusteeStatus}>}
}

interface MockGetListParams {
    pagination?: {page: number; perPage: number}
    filter?: {tally_session_id?: {value: {_in: Array<string>}}}
}

interface MockAction {
    icon: React.ReactElement<{title: string}>
    showAction?: () => boolean
}

let mockSessions: Array<MockTallySession>
let mockExecutions: Array<MockExecution>
let mockDisplayedSessions: Array<MockTallySession>
let mockIsTrustee: boolean
const mockSetTallyId = jest.fn()
const mockKeyRestoreQueryOptions = jest.fn()
const mockExecutionListOptions = jest.fn()

const mockNewestFirst = (sessions: Array<MockTallySession>) =>
    [...sessions].sort((a, b) => b.created_at.localeCompare(a.created_at))

const mockOperationName = (query: DocumentNode) => getOperationName(query)

jest.mock("react-admin", () => ({
    List: ({children}: {children?: React.ReactNode}) =>
        require("react").createElement("div", null, children),
    // Renders only the actions column, one row per tally the list is showing.
    DatagridConfigurable: ({children}: {children?: React.ReactNode}) => {
        const actionsField = require("react")
            .Children.toArray(children)
            .find(
                (child: React.ReactElement<{source?: string}>) => child.props.source === "actions"
            )
        return require("react").createElement(
            "div",
            null,
            mockDisplayedSessions.map((record) =>
                require("react").createElement(
                    "div",
                    {"key": record.id, "data-testid": record.id},
                    actionsField.props.render(record)
                )
            )
        )
    },
    FunctionField: () => null,
    TextField: () => null,
    TextInput: () => null,
    DateField: () => null,
    Button: () => null,
    useRecordContext: () => ({id: "event-1", status: {is_published: true}}),
    useListContext: () => ({data: mockDisplayedSessions}),
    useNotify: () => jest.fn(),
    useRefresh: () => jest.fn(),
    // Like react-admin with ra-data-hasura: a single page, 25 rows unless told otherwise.
    useGetList: (resource: string, params: MockGetListParams = {}, options?: unknown) => {
        const perPage = params.pagination?.perPage ?? 25
        if (resource === "sequent_backend_tally_session") {
            return {data: mockNewestFirst(mockSessions).slice(0, perPage)}
        }
        mockExecutionListOptions(options)
        const ids = params.filter?.tally_session_id?.value._in ?? []
        return {
            data: mockExecutions
                .filter((execution) => ids.includes(execution.tally_session_id))
                .slice(0, perPage),
        }
    },
}))
jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useMutation: () => [jest.fn()],
    useQuery: (query: DocumentNode, options: {skip?: boolean}) => {
        if (mockOperationName(query) !== "ListTallyKeyRestoreState") {
            return {data: undefined}
        }
        mockKeyRestoreQueryOptions(options)
        return {
            data: options.skip
                ? undefined
                : {
                      sequent_backend_tally_session: mockNewestFirst(mockSessions),
                      sequent_backend_tally_session_execution: mockExecutions,
                  },
        }
    },
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        trustee: "trustee-1",
        isAuthorized: (_: boolean, __: string, permission: string | Array<string>) =>
            mockIsTrustee && permission === "trustee-ceremony",
    }),
}))
jest.mock("../ElectionEvent/useKeysPermissions", () => ({
    useKeysPermissions: () => ({
        canAdminCeremony: false,
        canTrusteeCeremony: mockIsTrustee,
        canExportCeremony: false,
        canCreateCeremony: false,
        showTallyColumns: false,
    }),
}))
jest.mock("@/providers/ElectionEventTallyProvider", () => ({
    useElectionEventTallyStore: () => ({setTallyId: mockSetTallyId, setCreatingFlag: jest.fn()}),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-1"]}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_FAST_POLL_INTERVAL_MS: 5000},
    }),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
    Trans: ({children}: {children?: React.ReactNode}) =>
        require("react").createElement(require("react").Fragment, null, children),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({theme: {palette: {brandSuccess: "green"}}, IconButton: () => null, Dialog: () => null}),
    {virtual: true}
)
// Shows the tooltip of every action the row offers.
jest.mock("@/components/ActionButons", () => ({
    ActionsColumn: ({actions}: {actions: Array<MockAction>}) =>
        require("react").createElement(
            "span",
            null,
            actions
                .filter((action) => !action.showAction || action.showAction())
                .map((action) => action.icon.props.title)
                .join(", ")
        ),
}))
jest.mock("@/components/ListActions", () => ({ListActions: () => null}))
jest.mock("@/components/ElectionHeader", () => ({__esModule: true, default: () => null}))
jest.mock("@/components/TrusteeItems", () => ({TrusteeItems: () => null}))
jest.mock("@/components/StatusChip", () => ({StatusChip: () => null}))
jest.mock("@/components/ResetFilters", () => ({ResetFilters: () => null}))
jest.mock("@/components/StyledChip", () => ({StyledChip: () => null}))
jest.mock("@/components/styles/ResourceListStyles", () => ({
    ResourceListStyles: {EmptyBox: () => null},
}))
jest.mock("@fortawesome/free-solid-svg-icons", () => ({faPlus: {}}))
jest.mock("@mui/icons-material", () => ({Add: () => null}))
jest.mock("@mui/icons-material/CellTower", () => ({__esModule: true, default: () => null}))
jest.mock("@mui/icons-material/Description", () => ({__esModule: true, default: () => null}))
jest.mock("@mui/icons-material/Key", () => ({__esModule: true, default: () => null}))
jest.mock("@mui/icons-material/DoNotDisturbOn", () => ({__esModule: true, default: () => null}))

// Tallies tally-01 (oldest) to tally-NN (newest), all waiting for the trustee's key.
const tallySessions = (count: number): Array<MockTallySession> =>
    Array.from({length: count}, (_, index) => ({
        id: `tally-${String(index + 1).padStart(2, "0")}`,
        created_at: new Date(Date.UTC(2026, 0, index + 1)).toISOString(),
        execution_status: ITallyExecutionStatus.STARTED,
    }))

const execution = (tally_session_id: string, status: ITallyTrusteeStatus): MockExecution => ({
    tally_session_id,
    status: {trustees: [{name: TRUSTEE_NAME, status}]},
})

const rowActions = (tallySessionId: string) => screen.getByTestId(tallySessionId).textContent

const lastCallOptions = (mock: jest.Mock) => mock.mock.calls[mock.mock.calls.length - 1]?.[0]

const lastKeyRestoreQueryOptions = () => lastCallOptions(mockKeyRestoreQueryOptions)

const lastExecutionListOptions = () => lastCallOptions(mockExecutionListOptions)

const renderListTally = () => render(React.createElement(ListTally, {recordTally: RECORD_TALLY}))

describe("ListTally key restore for trustees", () => {
    beforeEach(() => {
        mockKeyRestoreQueryOptions.mockClear()
        mockExecutionListOptions.mockClear()
        mockSetTallyId.mockClear()
        mockIsTrustee = true
        mockSessions = tallySessions(30)
        mockExecutions = mockSessions.map((session) =>
            execution(session.id, ITallyTrusteeStatus.WAITING)
        )
        // Sorted by Created At ascending, the first page holds the oldest tallies.
        mockDisplayedSessions = mockSessions.slice(0, LIST_PAGE_SIZE)
    })

    it("offers the key upload on tallies older than the 25 newest of the event", () => {
        renderListTally()

        mockDisplayedSessions.forEach((session) => expect(rowActions(session.id)).toBe(ADD_KEY))
    })

    it("reads each row's trustee status from that row's own execution", () => {
        mockExecutions = [
            execution("tally-01", ITallyTrusteeStatus.KEY_RESTORED),
            ...mockExecutions.slice(1),
        ]

        renderListTally()

        expect(rowActions("tally-01")).toBe(VIEW)
        expect(rowActions("tally-02")).toBe(ADD_KEY)
    })

    it("keeps polling the executions of the listed tallies", () => {
        renderListTally()

        expect(lastExecutionListOptions()).toEqual(
            expect.objectContaining({enabled: true, refetchInterval: POLL_INTERVAL_MS})
        )
    })

    it("invites the trustee to a waiting tally older than the 25 newest of the event", () => {
        mockSessions = mockSessions.map((session, index) =>
            index === 0 ? session : {...session, execution_status: ITallyExecutionStatus.SUCCESS}
        )
        mockDisplayedSessions = []

        renderListTally()
        fireEvent.click(screen.getByText("click on the tally Key Action"))

        expect(mockSetTallyId).toHaveBeenCalledWith("tally-01", true)
    })

    it("loads the invitation state of the event as a trustee and keeps polling it", () => {
        renderListTally()

        expect(lastKeyRestoreQueryOptions()).toEqual(
            expect.objectContaining({
                variables: {tenantId: "tenant-1", electionEventId: "event-1"},
                skip: false,
                pollInterval: POLL_INTERVAL_MS,
                context: {headers: {"x-hasura-role": IPermissions.TRUSTEE_CEREMONY}},
            })
        )
    })

    it("loads no key restore state for users who are not trustees", () => {
        mockIsTrustee = false

        renderListTally()

        expect(lastKeyRestoreQueryOptions()).toEqual(expect.objectContaining({skip: true}))
        expect(lastExecutionListOptions()).toEqual(expect.objectContaining({enabled: false}))
    })
})
