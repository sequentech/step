/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import {ElectionTabs} from "./ElectionTabs"

const mockElection = {id: "election-1", election_event_id: "event-1", name: "Election 1"}

let mockDataTabMounts = 0

interface ElectionIdsProps {
    electionEventId?: string
    electionId?: string
    type?: string
}

function mockRenderIds(testId: string) {
    return ({electionEventId, electionId, type}: ElectionIdsProps) =>
        require("react").createElement(
            "div",
            {"data-testid": testId},
            [electionEventId, electionId, type].filter(Boolean).join("|")
        )
}

jest.mock("react-admin", () => ({
    useRecordContext: () => mockElection,
    useSidebarState: () => [true],
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        tenantId: "tenant-1",
        permissionLabels: [],
        isAuthorized: () => true,
    }),
}))
jest.mock("@sequentech/ui-core", () => ({EElectionEventLockedDown: {LOCKED_DOWN: "locked-down"}}), {
    virtual: true,
})
jest.mock("@/hooks/useAliasRenderer", () => ({
    useAliasRenderer: () => (record: {name?: string}) => record?.name,
}))
jest.mock("@/components/ElectionHeader", () => ({__esModule: true, default: () => null}))
jest.mock("@/components/styles/ResourceListStyles", () => ({
    ResourceListStyles: {EmptyBox: () => null},
}))
jest.mock("@/components/dashboard/election/Dashboard", () => ({
    __esModule: true,
    default: () => null,
}))
jest.mock("@/components/monitoring-dashboard/election/MonitoringDashboard", () => ({
    __esModule: true,
    default: () => null,
}))
jest.mock("./ElectionData", () => ({
    EditElectionData: () => {
        const react = require("react")
        react.useEffect(() => {
            mockDataTabMounts++
        }, [])
        return react.createElement("div", {"data-testid": "election-data"})
    },
}))
jest.mock("../ElectionEvent/EditElectionEventUsers", () => ({
    EditElectionEventUsers: mockRenderIds("voters-tab"),
}))
jest.mock("../Publish/Publish", () => ({Publish: mockRenderIds("publish-tab")}))
jest.mock("../ElectionEvent/EditElectionEventApprovals", () => ({
    EditElectionEventApprovals: mockRenderIds("approvals-tab"),
}))

const renderTabs = () => render(React.createElement(ElectionTabs))

const openTab = (label: string) => fireEvent.click(screen.getByRole("tab", {name: label}))

describe("ElectionTabs", () => {
    beforeEach(() => {
        mockDataTabMounts = 0
    })

    it("keeps the open tab mounted when the election tabs re-render", () => {
        const {rerender} = renderTabs()
        openTab("electionScreen.tabs.data")
        expect(screen.getByTestId("election-data")).toBeDefined()
        expect(mockDataTabMounts).toBe(1)

        rerender(React.createElement(ElectionTabs))
        rerender(React.createElement(ElectionTabs))

        expect(screen.getByTestId("election-data")).toBeDefined()
        expect(mockDataTabMounts).toBe(1)
    })

    it.each([
        ["electionScreen.tabs.voters", "voters-tab", "event-1|election-1"],
        ["electionScreen.tabs.publish", "publish-tab", "event-1|election-1|election"],
        ["electionScreen.tabs.approvals", "approvals-tab", "event-1|election-1"],
    ])("passes the election ids of the record to the %s tab", (label, testId, expected) => {
        renderTabs()

        openTab(label)

        expect(screen.getByTestId(testId).textContent).toBe(expected)
    })
})
