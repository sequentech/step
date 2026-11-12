/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import {EMonitoringCapability, EMonitoringMode} from "@/components/monitoring/types"
import {ElectionEventDashboardTab} from "./ElectionEventDashboardTab"

const PORTAL = "https://vote.example.org"
const TENANT = "11111111-1111-4111-8111-111111111111"
const EVENT = "22222222-2222-4222-8222-222222222222"
const OTHER_EVENT = "33333333-3333-4333-8333-333333333333"

let mockEvent: {id: string; voting_channels: {online: boolean; kiosk: boolean}}
let mockView = "GRANTED"
let mockList: {mode: string; dashboards: Array<{id: string}>} | undefined
let mockDashboard = "overview"

jest.mock("react-admin", () => ({
    useRecordContext: () => mockEvent,
}))
jest.mock("@/providers/TenantContextProvider", () => ({
    useTenantStore: () => [TENANT, () => undefined],
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {VOTING_PORTAL_URL: "https://vote.example.org"},
    }),
}))
// sequent-core's get_auth_url: `{base}/tenant/{tenant}/event/{event}/{action}`.
jest.mock("@/services/UrlGeneration", () => ({
    getAuthUrl: (base: string, tenant: string, event: string, action: string) =>
        `${base}/tenant/${tenant}/event/${event}/${action}`,
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useMutation: () => [jest.fn(), {loading: false}],
    useQuery: (_query: string, options?: {skip?: boolean}) =>
        options?.skip
            ? {data: undefined, loading: false}
            : {data: mockList ? {monitoringListDashboards: mockList} : undefined, loading: false},
}))
jest.mock("@/components/monitoring/useMonitoringPermissions", () => ({
    EMonitoringLock: {OPEN: "OPEN"},
    monitoringLock: () => "OPEN",
    useMonitoringPermissions: () => ({view: mockView, configure: "GRANTED"}),
}))
jest.mock("@/components/monitoring/editor/useMonitoringTabEditor", () => ({
    useMonitoringTabEditor: () => ({actions: undefined, element: null}),
}))
jest.mock("@/components/monitoring/MonitoringProvider", () => ({
    MonitoringProvider: ({children}: {children: React.ReactNode}) => children,
}))
jest.mock("@/components/monitoring/MonitoringDashboard", () => ({
    MonitoringDashboard: () => require("react").createElement("p", null, mockDashboard),
}))
jest.mock("@/components/dashboard/election-event/Dashboard", () => ({
    __esModule: true,
    default: () => require("react").createElement("p", null, "standard dashboard"),
}))
jest.mock("@/components/dashboard/FailedSealsBanner", () => ({FailedSealsBanner: () => null}))
jest.mock("@/resources/ElectionEvent/ListIpAddress", () => ({
    ListIpAddress: () => require("react").createElement("p", null, "ip addresses"),
}))

const urls = (event: string, kiosk: boolean) => [
    `${PORTAL}/tenant/${TENANT}/event/${event}/login`,
    `${PORTAL}/tenant/${TENANT}/event/${event}/enroll`,
    ...(kiosk ? [`${PORTAL}/tenant/${TENANT}/event/${event}/login?kiosk`] : []),
]
const links = () => screen.queryAllByRole("link").map((link) => link.getAttribute("href"))
const tab = () => (
    <ElectionEventDashboardTab refreshRef={{current: null}} handleChildMount={() => undefined} />
)

beforeEach(() => {
    mockEvent = {id: EVENT, voting_channels: {online: true, kiosk: false}}
    mockView = EMonitoringCapability.GRANTED
    mockList = {mode: EMonitoringMode.CONFIGURED, dashboards: [{id: "overview"}]}
    mockDashboard = "overview"
})

describe("event dashboard voter links", () => {
    it("are under the configured dashboards, whichever is selected", async () => {
        const {rerender} = render(tab())
        await screen.findByText("overview")
        expect(links()).toEqual(urls(EVENT, false))
        mockDashboard = "turnout"
        rerender(tab())
        await screen.findByText("turnout")
        expect(links()).toEqual(urls(EVENT, false))
    })

    it("are under configured monitoring that has no dashboards", async () => {
        mockList = {mode: EMonitoringMode.CONFIGURED, dashboards: []}
        render(tab())
        await screen.findByText("monitoring.noDashboards")
        expect(links()).toEqual(urls(EVENT, false))
    })

    it("are shown once under the standard dashboard in legacy mode", async () => {
        mockList = {mode: EMonitoringMode.LEGACY, dashboards: []}
        render(tab())
        await screen.findByText("standard dashboard")
        expect(links()).toEqual(urls(EVENT, false))
    })

    it("are shown once to a viewer without monitoring-view", async () => {
        mockView = EMonitoringCapability.DENIED
        render(tab())
        await screen.findByText("standard dashboard")
        expect(links()).toEqual(urls(EVENT, false))
    })

    it("follow the shown event and its kiosk channel", async () => {
        mockEvent = {id: OTHER_EVENT, voting_channels: {online: true, kiosk: true}}
        render(tab())
        await screen.findByText("overview")
        expect(links()).toEqual(urls(OTHER_EVENT, true))
    })

    it("come before the IP address list", async () => {
        render(tab())
        await screen.findByText("overview")
        const login = screen.getByRole("link", {name: "dashboard.voterLoginURL"})
        const ips = screen.getByText("ip addresses")
        expect(login.compareDocumentPosition(ips) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    })
})
