/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import {ElectionEventTabs} from "./ElectionEventTabs"
import {IPermissions} from "@/types/keycloak"

let mockRoles = new Set<string>()
let mockLocked = true
jest.mock("react-admin", () => ({
    useRecordContext: () => ({
        id: "event",
        presentation: {locked_down: mockLocked ? "locked-down" : "not-locked-down"},
    }),
    useSidebarState: () => [false],
    RecordContextProvider: ({children}: {children: React.ReactNode}) => children,
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        tenantId: "tenant",
        isAuthorized: (_: boolean, _tenant: string, roles: string | string[]) =>
            (Array.isArray(roles) ? roles : [roles]).some((role) => mockRoles.has(role)),
    }),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-router-dom", () => ({
    useLocation: () => ({pathname: "/sequent_backend_election_event/event", search: ""}),
    useNavigate: () => jest.fn(),
}))
jest.mock("uuid", () => ({v4: () => "stable"}))
jest.mock("@sequentech/ui-core", () => ({
    EElectionEventLockedDown: {LOCKED_DOWN: "locked-down", NOT_LOCKED_DOWN: "not-locked-down"},
    EVoterCertificatePolicy: {ENABLED: "enabled"},
}))
jest.mock("@/components/ElectionHeader", () => ({__esModule: true, default: () => null}))
jest.mock("@/providers/EventTimeZoneProvider", () => ({
    EventTimeZoneProvider: ({children}: {children: React.ReactNode}) => children,
}))
jest.mock("@/components/dashboard/FailedSealsBanner", () => ({FailedSealsBanner: () => null}))
jest.mock("@/providers/ElectionEventTallyProvider", () => ({
    useElectionEventTallyStore: () => ({setTallyId: jest.fn()}),
}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => () => "Event"}))
jest.mock("@/components/monitoring/useMonitoringPermissions", () => ({
    monitoringLock: () => "locked",
}))
jest.mock("@/components/Tabs", () => ({
    Tabs: ({elements}: {elements: {label: string; component: React.FC}[]}) => (
        <div>
            {elements.map(({label, component: Component}) => (
                <div key={label}>
                    <button role="tab">{label}</button>
                    {mockLocked && label === "electionEventScreen.tabs.data" ? <Component /> : null}
                </div>
            ))}
        </div>
    ),
}))

beforeEach(() => {
    mockRoles = new Set()
    mockLocked = true
})
it("keeps permitted tally and publication operations visible during lockdown while configuration tabs remain hidden", () => {
    mockRoles = new Set([
        IPermissions.TALLY_START,
        IPermissions.ELECTION_EVENT_TALLY_TAB,
        IPermissions.ELECTION_EVENT_PUBLISH_TAB,
        IPermissions.ELECTION_EVENT_DATA_TAB,
        IPermissions.ELECTION_EVENT_SCHEDULED_TAB,
    ])
    render(<ElectionEventTabs />)
    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
        "electionEventScreen.tabs.data",
        "electionEventScreen.tabs.tally",
        "electionEventScreen.tabs.publish",
    ])
})
it.each([
    {roles: [IPermissions.TALLY_START]},
    {roles: [IPermissions.ELECTION_EVENT_TALLY_TAB]},
    {roles: []},
])("does not grant tally access without both existing permission checks: %j", ({roles}) => {
    mockRoles = new Set(roles)
    render(<ElectionEventTabs />)
    expect(screen.queryByRole("tab", {name: "electionEventScreen.tabs.tally"})).toBeNull()
})
it("shows the actual lockdown state without exposing an editable Data form", () => {
    mockRoles.add(IPermissions.ELECTION_EVENT_DATA_TAB)
    render(<ElectionEventTabs />)
    expect(
        screen.getByText("electionEventScreen.field.lockdownState.options.locked-down")
    ).toBeTruthy()
    expect(screen.queryByRole("combobox")).toBeNull()
})
it("continues to show Data before lockdown with its existing permission", () => {
    mockLocked = false
    mockRoles.add(IPermissions.ELECTION_EVENT_DATA_TAB)
    render(<ElectionEventTabs />)
    expect(screen.getByRole("tab", {name: "electionEventScreen.tabs.data"})).toBeTruthy()
})
