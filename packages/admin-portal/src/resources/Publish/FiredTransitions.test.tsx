/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {cleanup, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {testI18n} from "@/components/timezones/__fixtures__/testI18n"
import type {IRetainedSignedClose} from "@/types/lifecycle"
import {FiredTransitions} from "./FiredTransitions"
import {closedAuthorized} from "./__stories__/FiredOutcomeFixture"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/services/timeZones"),
    ...jest.requireActual("../../../../ui-core/src/services/eventTimeZones"),
    ...jest.requireActual("../../../../ui-core/src/types/ScheduledOutcome"),
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
}))
const mockI18n = testI18n()
jest.mock("react-i18next", () => ({useTranslation: () => ({t: mockI18n.t, i18n: mockI18n})}))
jest.mock("react-admin", () => ({
    useGetList: () => ({data: mockEvents}),
    useGetOne: () => ({data: {id: "post"}}),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/components/timezones/useTimeZoneContext", () => ({
    useTimeZoneContext: () => ({zoneOf: () => "Asia/Dubai"}),
}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => () => "Office"}))
jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useQuery: () => ({data: {get_scheduled_outcomes: {outcomes: [], retained_closes: mockCloses}}}),
}))
let mockEvents: Array<Record<string, unknown>> = []
let mockCloses: Array<IRetainedSignedClose> = []
const close = (): IRetainedSignedClose => ({
    scheduled_event_id: "close",
    election_id: "post",
    fingerprint: "c4f1e0",
    scheduled_at: "2028-05-08T11:00:00Z",
    channels: ["ONLINE"],
    authorized_by: {request_id: "approval", code: "K7Q-2M", signers: ["Example Officer"]},
})
const show = () => render(<FiredTransitions electionEventId="event" electionId="post" />)
afterEach(cleanup)

describe("retained signed closes in Publish", () => {
    beforeEach(() => {
        mockEvents = []
        mockCloses = []
    })
    it("shows the exact retained deadline after its live row was deleted", () => {
        mockCloses = [close()]
        show()
        expect(screen.getByText("Signed close deadline")).toBeVisible()
        expect(screen.getByText(/Office:.*15:00.*K7Q-2M/)).toBeVisible()
        expect(screen.getByText(/changed or removed/)).toBeVisible()
    })
    it("describes a processing marker without claiming voting closed then", () => {
        mockCloses = [{...close(), fired_at: "2028-05-08T11:05:00Z"}]
        show()
        expect(screen.getByRole("heading", {name: /processed at.*15:05/})).toBeVisible()
        expect(screen.queryByTestId("closed-on-schedule")).not.toBeInTheDocument()
        expect(screen.getByText(/actual changes/)).toBeVisible()
        expect(screen.queryByText(/Channels still covered/)).not.toBeInTheDocument()
    })
    it("deduplicates the same signed live record while retaining its actual result and deadline", () => {
        const post = {...closedAuthorized("post", "close"), authorized_by: close().authorized_by}
        mockEvents = [
            {
                id: "close",
                event_processor: "END_VOTING_PERIOD",
                event_payload: {election_id: "post"},
                annotations: {fired_outcome: {at: "2028-05-08T11:05:00Z", posts: [post]}},
            },
        ]
        mockCloses = [{...close(), fired_at: "2028-05-08T11:05:00Z"}]
        show()
        expect(screen.getByTestId("closed-on-schedule")).toBeVisible()
        expect(screen.queryByText(/Signed close deadline processed/)).not.toBeInTheDocument()
        expect(screen.getByText(/Signed deadline:.*15:00/)).toBeVisible()
    })
    it("keeps the recorded target and action after editing the live row", () => {
        const post = {...closedAuthorized("post", "close"), authorized_by: close().authorized_by}
        mockEvents = [
            {
                id: "close",
                event_processor: "START_ENROLLMENT_PERIOD",
                event_payload: {election_id: "other"},
                annotations: {fired_outcome: {at: "2028-05-08T11:05:00Z", posts: [post]}},
            },
        ]
        mockCloses = [{...close(), fired_at: "2028-05-08T11:05:00Z"}]
        show()
        expect(screen.getByTestId("closed-on-schedule")).toBeVisible()
        expect(screen.queryByTestId("opened-on-schedule")).not.toBeInTheDocument()
        expect(screen.queryByText(/Signed close deadline processed/)).not.toBeInTheDocument()
    })

    it("keeps a refused edited-row result beside the retained processed deadline", () => {
        const post = {
            ...closedAuthorized("post", "close"),
            outcome: "refused",
            fingerprint: "edited",
        }
        mockEvents = [
            {
                id: "close",
                event_processor: "END_VOTING_PERIOD",
                event_payload: {election_id: "post"},
                cron_config: {scheduled_date: "2028-05-08T17:00:00Z"},
                annotations: {fired_outcome: {at: "2028-05-08T11:05:00Z", posts: [post]}},
            },
        ]
        mockCloses = [{...close(), fired_at: "2028-05-08T11:05:00Z"}]
        show()
        expect(screen.getByTestId("closed-on-schedule")).toBeVisible()
        expect(screen.getByRole("heading", {name: /Signed close deadline processed/})).toBeVisible()
        expect(screen.getByText(/Office:.*15:00.*K7Q-2M/)).toBeVisible()
    })
})
