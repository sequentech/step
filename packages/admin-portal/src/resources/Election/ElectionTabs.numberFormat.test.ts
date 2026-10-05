/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {cleanup, render, screen} from "@testing-library/react"
import {ENumberFormatPolicy} from "@sequentech/ui-core"
import {ElectionTabs} from "./ElectionTabs"

const mockUseGetOne = jest.fn()

jest.mock("react-admin", () => ({
    useGetOne: (...args: unknown[]) => mockUseGetOne(...args),
    useRecordContext: () => ({id: "election", election_event_id: "event", presentation: {}}),
    useSidebarState: () => [true],
    RecordContextProvider: ({children}: {children: React.ReactNode}) => children,
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("uuid", () => ({v4: () => "publish-tab"}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthorized: () => true,
        permissionLabels: [],
    }),
}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => () => "Election"}))
jest.mock("@/components/ElectionHeader", () => () => null)
jest.mock("@/components/styles/ResourceListStyles", () => ({
    ResourceListStyles: {EmptyBox: "div"},
}))
jest.mock("@/components/dashboard/election/Dashboard", () => () => {
    const {formatNumber} = require("@sequentech/ui-core").useNumberFormat()
    return formatNumber(1234567)
})
jest.mock("../Publish/Publish", () => ({Publish: () => null}))
jest.mock("./ElectionData", () => ({EditElectionData: () => null}))
jest.mock("../ElectionEvent/EditElectionEventUsers", () => ({EditElectionEventUsers: () => null}))
jest.mock("../ElectionEvent/EditElectionEventApprovals", () => ({
    EditElectionEventApprovals: () => null,
}))
jest.mock("../TallySheet/TallySheetWizard", () => ({
    TallySheetWizard: () => null,
    WizardSteps: {List: 0},
}))
jest.mock("../TallySheet/ListTallySheet", () => ({ListTallySheet: () => null}))

afterEach(() => {
    cleanup()
    mockUseGetOne.mockReset()
})

describe("ElectionTabs", () => {
    it("writes the election's figures in its election event's number format", () => {
        mockUseGetOne.mockReturnValue({
            data: {
                id: "event",
                presentation: {number_format_policy: ENumberFormatPolicy.PERIOD_COMMA},
            },
        })

        render(React.createElement(ElectionTabs))

        expect(screen.getByText("1.234.567")).toBeTruthy()
        expect(mockUseGetOne).toHaveBeenCalledWith("sequent_backend_election_event", {
            id: "event",
        })
    })

    it("groups figures with commas until the election event loads", () => {
        mockUseGetOne.mockReturnValue({data: undefined})

        render(React.createElement(ElectionTabs))

        expect(screen.getByText("1,234,567")).toBeTruthy()
    })
})
