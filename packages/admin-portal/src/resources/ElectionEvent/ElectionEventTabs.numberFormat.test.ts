/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {cleanup, getDefaultNormalizer, render, screen} from "@testing-library/react"
import {ENumberFormatPolicy} from "@sequentech/ui-core"
import {ElectionEventTabs} from "./ElectionEventTabs"

let mockElectionEvent: {id: string; presentation?: unknown} = {id: "event"}

jest.mock("react-admin", () => ({
    useRecordContext: () => mockElectionEvent,
    useSidebarState: () => [true],
    RecordContextProvider: ({children}: {children: React.ReactNode}) => children,
}))
jest.mock("react-router-dom", () => ({
    useNavigate: () => () => undefined,
    useLocation: () => ({pathname: "/sequent_backend_election_event/event", search: ""}),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("uuid", () => ({v4: () => "tab"}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true}),
}))
jest.mock("@/providers/ElectionEventTallyProvider", () => ({
    useElectionEventTallyStore: () => ({setTallyId: () => undefined}),
}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => () => "Event"}))
jest.mock("@/components/ElectionHeader", () => () => null)
jest.mock("@/components/dashboard/election-event/Dashboard", () => () => {
    const {formatNumber} = require("@sequentech/ui-core").useNumberFormat()
    return formatNumber(1234567)
})

afterEach(cleanup)

describe("ElectionEventTabs", () => {
    it("writes the event's figures in its number format", async () => {
        mockElectionEvent = {
            id: "event",
            presentation: {number_format_policy: ENumberFormatPolicy.SPACE_COMMA},
        }

        render(React.createElement(ElectionEventTabs))

        expect(
            await screen.findByText("1\u00a0234\u00a0567", {
                normalizer: getDefaultNormalizer({collapseWhitespace: false}),
            })
        ).toBeTruthy()
    })

    it("groups figures with commas for events saved before the policy existed", async () => {
        mockElectionEvent = {id: "event", presentation: {}}

        render(React.createElement(ElectionEventTabs))

        expect(await screen.findByText("1,234,567")).toBeTruthy()
    })
})
