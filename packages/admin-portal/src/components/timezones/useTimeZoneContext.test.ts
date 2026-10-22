// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {IElectionEventPresentation} from "@sequentech/ui-core"
import {timeZoneContextOf} from "./useTimeZoneContext"
import {madridConfiguration, overseasConfiguration} from "./__fixtures__/configurations"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/services/eventTimeZones"),
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
}))
jest.mock("react-admin", () => ({}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))

describe.each([
    ["the overseas preset", overseasConfiguration()],
    ["the Madrid association", madridConfiguration()],
])("timeZoneContextOf under %s", (_name, configuration) => {
    const event = configuration.presentation as IElectionEventPresentation
    const elections = configuration.elections.map(({id, timezone}) => ({
        id,
        presentation: {timezone},
    }))
    const context = timeZoneContextOf(event, elections)
    const primary = event.timezones!.primary

    it("gives each election its own configured zone, else the primary", () => {
        for (const election of configuration.elections) {
            expect(context.zoneOf(election.id)).toBe(election.timezone ?? primary)
        }
    })

    it("gives event-wide rows the primary", () => {
        expect(context.zoneOf(null)).toBe(primary)
    })

    it("lists the configured zones and the logs policy", () => {
        expect(context.configured).toEqual(event.timezones!.configured)
        expect(context.logs).toBe(event.timezones!.logs)
    })

    it("ignores an election zone the event doesn't configure", () => {
        const [first] = configuration.elections
        const unconfigured = timeZoneContextOf(event, [
            {id: first.id, presentation: {timezone: "Antarctica/Troll"}},
        ])
        expect(unconfigured.zoneOf(first.id)).toBe(primary)
    })
})

it("uses UTC when nothing is configured", () => {
    const context = timeZoneContextOf(undefined, [
        {id: "a", presentation: {timezone: "Asia/Dubai"}},
    ])
    expect(context.primary).toBe("UTC")
    expect(context.configured).toEqual(["UTC"])
    expect(context.zoneOf("a")).toBe("UTC")
})
