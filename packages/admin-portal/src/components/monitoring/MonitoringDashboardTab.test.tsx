/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {MonitoringDashboardTab} from "./MonitoringDashboardTab"
import {EMonitoringCapability, EMonitoringMode} from "./types"

let mockView = "GRANTED"
let mockConfigure = "GRANTED"
let mockSaving = false
const mockSetMode = jest.fn()
let mockList: {mode: string; dashboards: Array<{id: string}>} | undefined
let mockError: Error | undefined

jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useMutation: () => [mockSetMode, {loading: mockSaving}],
    useQuery: (_query: string, options?: {skip?: boolean}) =>
        options?.skip
            ? {data: undefined, loading: false}
            : {
                  data: mockList ? {monitoringListDashboards: mockList} : undefined,
                  loading: false,
                  error: mockError,
              },
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("./useMonitoringPermissions", () => ({
    EMonitoringLock: {OPEN: "OPEN"},
    useMonitoringPermissions: () => ({view: mockView, configure: mockConfigure}),
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
const FOOTER = "forensic review"

const renderTab = () =>
    render(
        <MonitoringDashboardTab
            electionEventId="event"
            electionId="election"
            legacy={<p>{LEGACY}</p>}
            below={<p>{SLOT}</p>}
            footer={<p>{FOOTER}</p>}
        />
    )

beforeEach(() => {
    mockView = EMonitoringCapability.GRANTED
    mockConfigure = EMonitoringCapability.GRANTED
    mockSaving = false
    mockSetMode.mockReset().mockResolvedValue({data: {monitoringSetMode: {generation: 1}}})
    mockList = {mode: EMonitoringMode.CONFIGURED, dashboards: [{id: "overview"}]}
    mockError = undefined
})

describe("MonitoringDashboardTab slot", () => {
    it("keeps the footer available while dashboards load", () => {
        mockList = undefined
        renderTab()
        expect(screen.getByText(FOOTER)).toBeTruthy()
        expect(screen.getByRole("progressbar")).toBeTruthy()
        expect(screen.queryByText(SLOT)).toBeNull()
    })

    it("renders the slot under the dashboards when monitoring is configured", () => {
        renderTab()
        expect(screen.getByText(FOOTER)).toBeTruthy()
        expect(screen.getByText("dashboards")).toBeTruthy()
        expect(screen.getByText(SLOT)).toBeTruthy()
        expect(screen.queryByText(LEGACY)).toBeNull()
    })

    it("renders the slot when monitoring is configured without dashboards", () => {
        mockList = {mode: EMonitoringMode.CONFIGURED, dashboards: []}
        renderTab()
        expect(screen.getByText(FOOTER)).toBeTruthy()
        expect(screen.getByText("monitoring.noDashboards")).toBeTruthy()
        expect(screen.getByText(SLOT)).toBeTruthy()
    })

    it("renders no slot in legacy mode", () => {
        mockList = {mode: EMonitoringMode.LEGACY, dashboards: []}
        renderTab()
        expect(screen.getByText(FOOTER)).toBeTruthy()
        expect(screen.getByText(LEGACY)).toBeTruthy()
        expect(screen.queryByText(SLOT)).toBeNull()
    })

    it("renders no slot when monitoring can't be loaded", () => {
        mockError = new Error("unavailable")
        renderTab()
        expect(screen.getByText(FOOTER)).toBeTruthy()
        expect(screen.getByText(LEGACY)).toBeTruthy()
        expect(screen.queryByText(SLOT)).toBeNull()
    })

    it("renders no slot for a viewer without monitoring-view", () => {
        mockView = EMonitoringCapability.DENIED
        renderTab()
        expect(screen.getByText(FOOTER)).toBeTruthy()
        expect(screen.getByText(LEGACY)).toBeTruthy()
        expect(screen.queryByText(SLOT)).toBeNull()
    })
})

describe("monitoring mode switch", () => {
    it("enables a legacy event through the non-destructive mode action", async () => {
        mockList = {mode: EMonitoringMode.LEGACY, dashboards: []}
        renderTab()
        fireEvent.click(screen.getByRole("switch"))
        await waitFor(() =>
            expect(mockSetMode).toHaveBeenCalledWith({
                variables: {electionEventId: "event", mode: EMonitoringMode.CONFIGURED},
            })
        )
    })

    it("disables configured dashboards without resetting their documents", async () => {
        renderTab()
        const control = screen.getByRole("switch") as HTMLInputElement
        expect(control.checked).toBe(true)
        fireEvent.click(control)
        await waitFor(() =>
            expect(mockSetMode).toHaveBeenCalledWith({
                variables: {electionEventId: "event", mode: EMonitoringMode.LEGACY},
            })
        )
    })

    it.each(["permission", "saving"])("disables the switch while blocked by %s", (reason) => {
        if (reason === "permission") mockConfigure = EMonitoringCapability.DENIED
        else mockSaving = true
        renderTab()
        expect((screen.getByRole("switch") as HTMLInputElement).disabled).toBe(true)
        expect(mockSetMode).not.toHaveBeenCalled()
    })

    it("keeps the previous mode and shows an error after a refused change", async () => {
        mockSetMode.mockRejectedValue(new Error("refused"))
        renderTab()
        fireEvent.click(screen.getByRole("switch"))
        await screen.findByText("monitoring.modeChangeError")
        expect((screen.getByRole("switch") as HTMLInputElement).checked).toBe(true)
    })

    it("does not offer a switch without monitoring view permission", () => {
        mockView = EMonitoringCapability.DENIED
        renderTab()
        expect(screen.queryByRole("switch")).toBeNull()
        expect(mockSetMode).not.toHaveBeenCalled()
    })
})
