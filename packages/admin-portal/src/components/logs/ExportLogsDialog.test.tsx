/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ExportLogsDialog} from "./ExportLogsDialog"
import {ELogScope, type ILogTarget} from "./logScope"
import {testI18n} from "../timezones/__fixtures__/testI18n"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/services/timeZones"),
}))
const mockI18n = testI18n()
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: mockI18n.t, i18n: mockI18n}),
}))
const mockExport = jest.fn(async () => ({
    data: {export_election_event_logs: {task_execution: {id: "export-task"}}},
}))
const mockTenantExport = jest.fn(async () => ({
    data: {export_tenant_logs: {task_execution: {id: "tenant-export-task"}}},
}))
const mockRoles: Record<string, string> = {}
jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useMutation: (
        document: {definitions: Array<{name?: {value: string}}>},
        options: {context: {headers: Record<string, string>}}
    ) => {
        const name = document.definitions[0].name?.value ?? ""
        mockRoles[name] = options.context.headers["x-hasura-role"]
        return [name === "ExportTenantLogs" ? mockTenantExport : mockExport]
    },
}))
const mockLists = jest.fn((resource: string, _params: unknown, options: {enabled: boolean}) => ({
    data: !options.enabled
        ? undefined
        : resource === "sequent_backend_area"
          ? [{id: "area-1", name: "North"}]
          : [{id: "election-1"}],
}))
jest.mock("react-admin", () => ({
    useGetList: (resource: string, params: unknown, options: {enabled: boolean}) =>
        mockLists(resource, params, options),
}))
jest.mock("@/hooks/useAliasRenderer", () => ({
    useAliasRenderer: () => (item: {id: string}) => `Election ${item.id}`,
}))
const mockAddWidget = jest.fn((_type: string, _download: boolean) => ({identifier: "widget"}))
const mockSetTask = jest.fn()
jest.mock("@/providers/WidgetsContextProvider", () => ({
    useWidgetStore: () => [mockAddWidget, mockSetTask, jest.fn()],
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Dialog: ({children, ok, okEnabled, handleClose}: any) => (
            <div>
                {children}
                <button disabled={!okEnabled()} onClick={() => handleClose(true)}>
                    {ok}
                </button>
            </div>
        ),
    }),
    {virtual: true}
)

beforeEach(() => jest.clearAllMocks())
const EVENT: ILogTarget = {kind: ELogScope.EVENT, electionEventId: "event"}
const TENANT: ILogTarget = {kind: ELogScope.TENANT}
const mount = (log: ILogTarget = EVENT) =>
    render(<ExportLogsDialog log={log} open onClose={jest.fn()} zones={["America/New_York"]} />)
const enter = (name: string | RegExp, value: string) =>
    fireEvent.change(screen.getByLabelText(name), {target: {value}})
const choose = (label: string, option: string) => {
    fireEvent.mouseDown(screen.getByRole("combobox", {name: label}))
    fireEvent.click(within(screen.getByRole("listbox")).getByText(option))
}
const exportButton = () => screen.getByRole("button", {name: "Export"})
const WHOLE_RANGE = {createdFrom: null, createdTo: null, timeZone: "America/New_York"}

it("refuses a non-existent bound instead of shifting the requested export range", () => {
    mount()
    enter("From", "2028-03-12T02:30")
    expect(exportButton()).toBeDisabled()
    expect(screen.getByText(/does not exist/)).toBeVisible()
    fireEvent.click(exportButton())
    expect(mockExport).not.toHaveBeenCalled()
})

it("explains an overlap, exports its earlier occurrence, and preserves an empty other bound", async () => {
    mount()
    enter("From", "2028-11-05T01:30")
    expect(screen.getByText(/happens twice/)).toBeVisible()
    fireEvent.click(exportButton())
    await waitFor(() =>
        expect(mockExport).toHaveBeenCalledWith({
            variables: {
                electionEventId: "event",
                format: "CSV",
                scope: "ALL",
                electionId: null,
                areaId: null,
                username: null,
                createdFrom: "2028-11-05T05:30:00Z",
                createdTo: null,
                timeZone: "America/New_York",
            },
        })
    )
    expect(mockTenantExport).not.toHaveBeenCalled()
    expect(mockRoles.ExportElectionEventLogs).toBe("logs-export")
    expect(mockAddWidget).toHaveBeenCalledWith("EXPORT_ACTIVITY_LOGS_REPORT", true)
})

it("offers CSV, SQL and PDF, and says the export is recorded", () => {
    mount()
    expect(screen.getAllByRole("radio").map((radio) => (radio as HTMLInputElement).value)).toEqual([
        "CSV",
        "SQL",
        "PDF",
    ])
    expect(screen.getByText("The export itself is recorded in this log.")).toBeVisible()
    expect(screen.queryByText("Rows from election events name their log.")).toBeNull()
})

it("exports one election of an event once it is chosen", async () => {
    mount()
    choose("Scope", "One election")
    expect(exportButton()).toBeDisabled()
    choose("Election", "Election election-1")
    fireEvent.click(screen.getByRole("radio", {name: "SQL"}))
    fireEvent.click(exportButton())
    await waitFor(() =>
        expect(mockExport).toHaveBeenCalledWith({
            variables: {
                electionEventId: "event",
                format: "SQL",
                scope: "ELECTION",
                electionId: "election-1",
                areaId: null,
                username: null,
                ...WHOLE_RANGE,
            },
        })
    )
    expect(mockLists).toHaveBeenCalledWith(
        "sequent_backend_election",
        expect.objectContaining({filter: {election_event_id: "event"}}),
        {enabled: true}
    )
})

it("exports one area or one account of an event", async () => {
    mount()
    choose("Scope", "One area")
    choose("Area", "North")
    fireEvent.click(exportButton())
    await waitFor(() =>
        expect(mockExport).toHaveBeenLastCalledWith({
            variables: expect.objectContaining({scope: "AREA", areaId: "area-1", electionId: null}),
        })
    )
    choose("Scope", "One account")
    expect(exportButton()).toBeDisabled()
    enter(/^Username/, "rosa.marin")
    fireEvent.click(exportButton())
    await waitFor(() =>
        expect(mockExport).toHaveBeenLastCalledWith({
            variables: expect.objectContaining({
                scope: "ACCOUNT",
                username: "rosa.marin",
                areaId: null,
            }),
        })
    )
    // An event's account export has no election events to add.
    expect(screen.queryByRole("checkbox")).toBeNull()
})

it("exports the whole tenant log with the tenant's own role and task", async () => {
    mount(TENANT)
    expect(screen.getByRole("combobox", {name: "Scope"})).toHaveTextContent("The whole tenant log")
    expect(screen.queryByLabelText(/Username/)).toBeNull()
    fireEvent.click(exportButton())
    await waitFor(() =>
        expect(mockTenantExport).toHaveBeenCalledWith({
            variables: {format: "CSV", scope: "TENANT_LOG", username: null, ...WHOLE_RANGE},
        })
    )
    expect(mockExport).not.toHaveBeenCalled()
    expect(mockRoles.ExportTenantLogs).toBe("tenant-logs-export")
    expect(mockAddWidget).toHaveBeenCalledWith("EXPORT_TENANT_LOGS", true)
    await waitFor(() => expect(mockSetTask).toHaveBeenCalledWith("widget", "tenant-export-task"))
    expect(mockLists).not.toHaveBeenCalledWith(expect.anything(), expect.anything(), {
        enabled: true,
    })
})

it("needs a username for one account, and can add its election event entries", async () => {
    mount(TENANT)
    choose("Scope", "One account")
    expect(exportButton()).toBeDisabled()
    enter(/^Username/, "rosa.marin")
    expect(exportButton()).toBeEnabled()
    fireEvent.click(exportButton())
    await waitFor(() =>
        expect(mockTenantExport).toHaveBeenLastCalledWith({
            variables: {format: "CSV", scope: "ACCOUNT", username: "rosa.marin", ...WHOLE_RANGE},
        })
    )
    fireEvent.click(
        screen.getByRole("checkbox", {
            name: "Include this account's entries in every election event",
        })
    )
    expect(screen.getByText("Rows from election events name their log.")).toBeVisible()
    fireEvent.click(screen.getByRole("radio", {name: "PDF"}))
    fireEvent.click(exportButton())
    await waitFor(() =>
        expect(mockTenantExport).toHaveBeenLastCalledWith({
            variables: {
                format: "PDF",
                scope: "ACCOUNT_WITH_EVENTS",
                username: "rosa.marin",
                ...WHOLE_RANGE,
            },
        })
    )
})
