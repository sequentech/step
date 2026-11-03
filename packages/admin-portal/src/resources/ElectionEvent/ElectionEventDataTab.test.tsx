/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ElectionEventDataTab, LockedDownDataTab} from "./ElectionEventDataTab"
import {IPermissions} from "@/types/keycloak"

let mockEvent:
    | {id: string; tenant_id: string; presentation?: {locked_down?: string | null}}
    | undefined
let mockRoles: Set<string>
let mockTenant: string | null
const mockEditorRender = jest.fn()
jest.mock("react-admin", () => ({useRecordContext: () => mockEvent}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@sequentech/ui-core", () => ({
    EElectionEventLockedDown: {LOCKED_DOWN: "locked-down", NOT_LOCKED_DOWN: "not-locked-down"},
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => [mockTenant]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthorized: (_superAdmin: boolean, tenant: string, role: string) =>
            tenant === mockTenant && mockRoles.has(role),
    }),
}))
jest.mock("./EditElectionEventData", () => ({
    EditElectionEventData: () => {
        mockEditorRender()
        return (
            <form aria-label="Configuration">
                <button>Save configuration</button>
            </form>
        )
    },
}))
jest.mock("@/components/election-event/ReadinessCheckButton", () => ({
    ReadinessCheckButton: ({event}: {event: {id: string}}) => (
        <button>Readiness for {event.id}</button>
    ),
}))

beforeEach(() => {
    mockEditorRender.mockClear()
    mockTenant = "tenant-a"
    mockRoles = new Set([IPermissions.ELECTION_EVENT_DATA_TAB, IPermissions.READINESS_CHECK])
    mockEvent = {id: "official", tenant_id: "tenant-a", presentation: {locked_down: "locked-down"}}
})

it("renders only readiness during lockdown without ever mounting the editor", () => {
    render(<ElectionEventDataTab />)
    expect(screen.getByRole("button", {name: "Readiness for official"})).toBeVisible()
    expect(screen.queryByRole("form")).not.toBeInTheDocument()
    expect(mockEditorRender).not.toHaveBeenCalled()
})

it.each([IPermissions.READINESS_CHECK, IPermissions.ELECTION_EVENT_DATA_TAB])(
    "rejects missing %s both at the Data entry point and the isolated lockdown component",
    (role) => {
        mockRoles.delete(role)
        const {rerender} = render(<ElectionEventDataTab />)
        expect(screen.getByRole("alert")).toHaveTextContent("electionEventScreen.dataUnavailable")
        expect(screen.queryByRole("button")).not.toBeInTheDocument()
        rerender(<LockedDownDataTab />)
        expect(screen.getByRole("alert")).toBeVisible()
        expect(screen.queryByRole("button")).not.toBeInTheDocument()
        expect(mockEditorRender).not.toHaveBeenCalled()
    }
)

it("shows a loading state without mounting either action surface before the record arrives", () => {
    mockEvent = undefined
    render(<ElectionEventDataTab />)
    expect(screen.getByRole("progressbar", {name: "loading"})).toBeVisible()
    expect(screen.queryByRole("button")).not.toBeInTheDocument()
    expect(mockEditorRender).not.toHaveBeenCalled()
})

it.each([null, "another-tenant"])("rejects a missing or mismatched tenant (%s)", (tenant) => {
    mockTenant = tenant
    render(<ElectionEventDataTab />)
    expect(screen.getByRole("alert")).toBeVisible()
    expect(screen.queryByRole("button")).not.toBeInTheDocument()
    expect(mockEditorRender).not.toHaveBeenCalled()
})

it("fails closed for an unknown lockdown state", () => {
    mockEvent!.presentation = {locked_down: "unknown"}
    render(<ElectionEventDataTab />)
    expect(screen.getByRole("alert")).toBeVisible()
    expect(mockEditorRender).not.toHaveBeenCalled()
})

it.each([undefined, null, "not-locked-down"])(
    "keeps unlocked and legacy events editable (%s)",
    async (state) => {
        mockEvent!.presentation = {locked_down: state}
        mockRoles.delete(IPermissions.READINESS_CHECK)
        render(<ElectionEventDataTab />)
        expect(await screen.findByRole("form", {name: "Configuration"})).toBeVisible()
        expect(screen.queryByRole("button", {name: /Readiness/})).not.toBeInTheDocument()
    }
)

it("unmounts the editor when lockdown starts and restores it only after lockdown ends", async () => {
    mockEvent!.presentation = {locked_down: "not-locked-down"}
    const {rerender} = render(<ElectionEventDataTab />)
    expect(await screen.findByRole("form")).toBeVisible()
    mockEvent!.presentation = {locked_down: "locked-down"}
    rerender(<ElectionEventDataTab />)
    expect(screen.queryByRole("form")).not.toBeInTheDocument()
    expect(screen.getByRole("button", {name: /Readiness/})).toBeVisible()
    mockRoles.delete(IPermissions.READINESS_CHECK)
    rerender(<ElectionEventDataTab />)
    expect(screen.getByRole("alert")).toBeVisible()
    expect(screen.queryByRole("button")).not.toBeInTheDocument()
    mockEvent!.presentation = {locked_down: "not-locked-down"}
    rerender(<ElectionEventDataTab />)
    expect(await screen.findByRole("form")).toBeVisible()
})

it("does not expose the isolated lockdown action for an unlocked event", () => {
    mockEvent!.presentation = {locked_down: "not-locked-down"}
    render(<LockedDownDataTab />)
    expect(screen.getByRole("alert")).toBeVisible()
    expect(screen.queryByRole("button")).not.toBeInTheDocument()
})
