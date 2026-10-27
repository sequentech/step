/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {MonitoringDashboardTab} from "./MonitoringDashboardTab"
import {EMonitoringCapability, EMonitoringMode} from "./types"

let mockView = "GRANTED"
let mockConfigure = "DENIED"
let mockList: {mode: string; dashboards: Array<{id: string}>} | undefined
let mockError: Error | undefined
const mockRefetch = jest.fn()
const mockApi = {listPresets: jest.fn(), resetToPreset: jest.fn()}

jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useQuery: (_query: string, options?: {skip?: boolean}) =>
        options?.skip
            ? {data: undefined, loading: false}
            : {
                  data: mockList ? {monitoringListDashboards: mockList} : undefined,
                  loading: false,
                  error: mockError,
                  refetch: mockRefetch,
              },
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("./useMonitoringPermissions", () => ({
    EMonitoringLock: {OPEN: "OPEN"},
    useMonitoringPermissions: () => ({view: mockView, configure: mockConfigure}),
}))
jest.mock("./editor/useMonitoringEditorApi", () => ({
    useMonitoringEditorApi: () => mockApi,
}))
jest.mock("./editor/useMonitoringTabEditor", () => ({
    useMonitoringTabEditor: () => ({actions: undefined, element: null}),
}))
jest.mock("./MonitoringProvider", () => ({
    MonitoringProvider: ({children}: {children: React.ReactNode}) => children,
}))
jest.mock("./MonitoringDashboard", () => ({
    MonitoringDashboard: () => require("react").createElement("p", null, "dashboards"),
}))

const LEGACY = "legacy dashboard"
const SLOT = "under the dashboards"

const renderTab = () =>
    render(
        <MonitoringDashboardTab
            electionEventId="event"
            electionId="election"
            legacy={<p>{LEGACY}</p>}
            below={<p>{SLOT}</p>}
        />
    )

beforeEach(() => {
    mockView = EMonitoringCapability.GRANTED
    mockConfigure = EMonitoringCapability.DENIED
    mockList = {mode: EMonitoringMode.CONFIGURED, dashboards: [{id: "overview"}]}
    mockError = undefined
    mockRefetch.mockReset().mockResolvedValue({})
    mockApi.listPresets
        .mockReset()
        .mockResolvedValue([{id: "standard", title: "Standard monitoring", version: 1}])
    mockApi.resetToPreset.mockReset().mockResolvedValue({generation: 1})
})

describe("MonitoringDashboardTab slot", () => {
    it("renders the slot under the dashboards when monitoring is configured", () => {
        renderTab()
        expect(screen.getByText("dashboards")).toBeTruthy()
        expect(screen.getByText(SLOT)).toBeTruthy()
        expect(screen.queryByText(LEGACY)).toBeNull()
    })

    it("renders the slot when monitoring is configured without dashboards", () => {
        mockList = {mode: EMonitoringMode.CONFIGURED, dashboards: []}
        renderTab()
        expect(screen.getByText("monitoring.noDashboards")).toBeTruthy()
        expect(screen.getByText(SLOT)).toBeTruthy()
    })

    it("renders no slot in legacy mode", () => {
        mockList = {mode: EMonitoringMode.LEGACY, dashboards: []}
        renderTab()
        expect(screen.getByText(LEGACY)).toBeTruthy()
        expect(screen.queryByText(SLOT)).toBeNull()
    })

    it("renders no slot when monitoring can't be loaded", () => {
        mockError = new Error("unavailable")
        mockConfigure = EMonitoringCapability.GRANTED
        renderTab()
        expect(screen.getByText(LEGACY)).toBeTruthy()
        expect(screen.queryByText(SLOT)).toBeNull()
        expect(screen.queryByRole("button", {name: "monitoring.setup"})).toBeNull()
    })

    it("renders no slot for a viewer without monitoring-view", () => {
        mockView = EMonitoringCapability.DENIED
        mockConfigure = EMonitoringCapability.GRANTED
        renderTab()
        expect(screen.getByText(LEGACY)).toBeTruthy()
        expect(screen.queryByText(SLOT)).toBeNull()
        expect(screen.queryByRole("button", {name: "monitoring.setup"})).toBeNull()
    })

    it("lets an authorized holder choose a preset when configured dashboards are empty", () => {
        mockList = {mode: EMonitoringMode.CONFIGURED, dashboards: []}
        mockConfigure = EMonitoringCapability.GRANTED
        renderTab()
        expect(screen.getByRole("button", {name: "monitoring.setup"})).toBeTruthy()
        expect(mockApi.resetToPreset).not.toHaveBeenCalled()
    })

    it("sets up a legacy event only after the authorized holder confirms a preset", async () => {
        mockList = {mode: EMonitoringMode.LEGACY, dashboards: []}
        mockConfigure = EMonitoringCapability.GRANTED
        renderTab()
        fireEvent.click(screen.getByRole("button", {name: "monitoring.setup"}))
        await screen.findByRole("radio", {name: "Standard monitoring · v1"})
        expect(mockApi.resetToPreset).not.toHaveBeenCalled()
        fireEvent.click(screen.getByRole("button", {name: "monitoring.editor.reset.confirm"}))
        await waitFor(() => expect(mockRefetch).toHaveBeenCalledTimes(1))
        expect(mockApi.resetToPreset).toHaveBeenCalledTimes(1)
        expect(mockApi.resetToPreset).toHaveBeenCalledWith("standard")
    })

    it("leaves the event unchanged when setup is canceled", async () => {
        mockList = {mode: EMonitoringMode.LEGACY, dashboards: []}
        mockConfigure = EMonitoringCapability.GRANTED
        renderTab()
        fireEvent.click(screen.getByRole("button", {name: "monitoring.setup"}))
        await screen.findByRole("radio", {name: "Standard monitoring · v1"})
        fireEvent.click(screen.getByRole("button", {name: "monitoring.editor.reset.cancel"}))
        expect(mockApi.resetToPreset).not.toHaveBeenCalled()
        expect(mockRefetch).not.toHaveBeenCalled()
    })

    it.each(["DENIED", "UNKNOWN"])(
        "does not offer setup with %s configure permission",
        (permission) => {
            mockList = {mode: EMonitoringMode.LEGACY, dashboards: []}
            mockConfigure = permission
            renderTab()
            expect(screen.queryByRole("button", {name: "monitoring.setup"})).toBeNull()
            expect(mockApi.listPresets).not.toHaveBeenCalled()
        }
    )

    it("keeps a refused setup visible without claiming a successful reload", async () => {
        mockList = {mode: EMonitoringMode.LEGACY, dashboards: []}
        mockConfigure = EMonitoringCapability.GRANTED
        mockApi.resetToPreset.mockRejectedValue(new Error("The event is locked down"))
        renderTab()
        fireEvent.click(screen.getByRole("button", {name: "monitoring.setup"}))
        await screen.findByRole("radio", {name: "Standard monitoring · v1"})
        fireEvent.click(screen.getByRole("button", {name: "monitoring.editor.reset.confirm"}))
        await screen.findByText("monitoring.editor.reset.failed")
        expect(mockRefetch).not.toHaveBeenCalled()
        expect(screen.getByText(LEGACY)).toBeTruthy()
    })
})
