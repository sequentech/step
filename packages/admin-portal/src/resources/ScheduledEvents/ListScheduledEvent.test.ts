/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import ListScheduledEvents from "./ListScheduledEvent"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/types/VotingChannel"),
    ...jest.requireActual("../../../../ui-core/src/services/timeZones"),
    ...jest.requireActual("../../../../ui-core/src/services/eventTimeZones"),
    ...jest.requireActual("../../../../ui-core/src/types/ScheduledOutcome"),
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
}))
const mockDelete = jest.fn()
const mockNotify = jest.fn()
const mockRefresh = jest.fn()
const mockRecord = {
    id: "manual-kiosk-stop",
    election_event_id: "event",
    task_id: "custom-kiosk-stop",
    event_processor: "END_VOTING_PERIOD",
    event_payload: {voting_channels: ["KIOSK"]},
}
const mockT = (key: string) => key
jest.mock("@apollo/client", () => ({
    gql: (s: TemplateStringsArray) => s.join(""),
    useMutation: () => [mockDelete, {loading: false}],
    useQuery: () => ({data: undefined}),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: mockT})}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_POLL_INTERVAL_MS: 5000},
    }),
}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => () => "Election"}))
jest.mock("../ElectionEvent/useScheduledEventPermissions", () => ({
    useScheduledEventPermissions: () => ({
        canWriteScheduledEvent: true,
        canDeleteScheduledEvent: true,
    }),
}))
jest.mock("@/components/ElectionHeader", () => ({__esModule: true, default: () => null}))
jest.mock("@/components/ThreeStateDatagridHeader", () => ({ThreeStateDatagridHeader: () => null}))
jest.mock("@/resources/User/DownloadDocument", () => ({DownloadDocument: () => null}))
jest.mock("./ImportScheduleDrawer", () => ({ImportScheduleDrawer: () => null}))
jest.mock("@/components/ListActions", () => ({ListActions: () => null}))
jest.mock("@/components/styles/ResourceListStyles", () => ({
    ResourceListStyles: {Drawer: () => null},
}))
jest.mock("./CreateScheduledEvent", () => ({
    __esModule: true,
    default: () => null,
    EventProcessors: {END_VOTING_PERIOD: "END_VOTING_PERIOD"},
}))
jest.mock("@/components/ActionButons", () => ({
    ActionsColumn: ({actions}: {actions: Array<{label?: string; action: (id: string) => void}>}) =>
        require("react").createElement(
            "button",
            {
                onClick: () =>
                    actions
                        .find((action) => action.label === "common.label.delete")
                        ?.action(mockRecord.id),
            },
            "Delete kiosk"
        ),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
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
                      "Confirm delete"
                  )
                : null,
    }),
    {virtual: true}
)
jest.mock("react-admin", () => ({
    List: ({children}: React.PropsWithChildren) => children,
    DatagridConfigurable: ({children}: React.PropsWithChildren) => children,
    WrapperField: ({children}: React.PropsWithChildren) => children,
    TextField: () => null,
    FunctionField: () => null,
    SelectInput: () => null,
    TextInput: () => null,
    useGetList: () => ({data: []}),
    useGetOne: (_resource: string, {id}: {id: string}) => ({
        data: id === mockRecord.id ? mockRecord : undefined,
    }),
    useListContext: () => ({filterValues: {}}),
    useNotify: () => mockNotify,
    useRefresh: () => mockRefresh,
}))

beforeEach(() => {
    mockDelete.mockReset().mockResolvedValue({data: {manage_election_dates: {}}})
    mockNotify.mockClear()
    mockRefresh.mockClear()
})

it("archives the selected manual schedule by ID", async () => {
    render(React.createElement(ListScheduledEvents, {electionEventId: "event"}))
    fireEvent.click(screen.getByText("Delete kiosk"))
    fireEvent.click(screen.getByText("Confirm delete"))
    await waitFor(() =>
        expect(mockDelete).toHaveBeenCalledWith({
            variables: expect.objectContaining({
                scheduledEventId: "manual-kiosk-stop",
                electionEventId: "event",
                scheduledDate: undefined,
            }),
        })
    )
    await waitFor(() => expect(screen.queryByText("Confirm delete")).toBeNull())
})

it("keeps a rejected deletion open and displays its reason", async () => {
    const reason = "Scheduled event not found in this scope or already archived"
    mockDelete.mockResolvedValueOnce({data: {manage_election_dates: {error_msg: reason}}})
    render(React.createElement(ListScheduledEvents, {electionEventId: "event"}))
    fireEvent.click(screen.getByText("Delete kiosk"))
    fireEvent.click(screen.getByText("Confirm delete"))
    await waitFor(() => expect(mockNotify).toHaveBeenCalledWith(reason, {type: "error"}))
    expect(screen.queryByText("Confirm delete")).not.toBeNull()
    expect(mockRefresh).not.toHaveBeenCalled()
})
