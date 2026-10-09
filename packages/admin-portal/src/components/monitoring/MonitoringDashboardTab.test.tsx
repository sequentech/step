/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import {MonitoringDashboardTab} from "./MonitoringDashboardTab"
import {EMonitoringCapability, EMonitoringMode} from "./types"

let mockView = "GRANTED"
let mockList: {mode: string; dashboards: Array<{id: string}>} | undefined
let mockError: Error | undefined

jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
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
    useMonitoringPermissions: () => ({view: mockView, configure: "DENIED"}),
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
    mockList = {mode: EMonitoringMode.CONFIGURED, dashboards: [{id: "overview"}]}
    mockError = undefined
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
        renderTab()
        expect(screen.getByText(LEGACY)).toBeTruthy()
        expect(screen.queryByText(SLOT)).toBeNull()
    })

    it("renders no slot for a viewer without monitoring-view", () => {
        mockView = EMonitoringCapability.DENIED
        renderTab()
        expect(screen.getByText(LEGACY)).toBeTruthy()
        expect(screen.queryByText(SLOT)).toBeNull()
    })
})
