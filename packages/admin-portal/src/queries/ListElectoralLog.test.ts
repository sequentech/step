// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {electoralLogInstantFilters} from "./ListElectoralLog"

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../ui-core/src/services/timeZones")
)

const at = (iso: string) => new Date(iso).getTime()

describe("the electoral log range filter", () => {
    // Two configurations: a Manila primary (+08:00) and a Madrid one (+02:00 in April).
    it.each([
        ["Asia/Manila", "2028-04-07T16:00:00Z", "2028-04-09T15:59:00Z"],
        ["Europe/Madrid", "2028-04-07T22:00:00Z", "2028-04-09T21:59:00Z"],
    ])("reads the wall times in %s", (zone, from, to) => {
        const filter = electoralLogInstantFilters({
            election_event_id: "event",
            created_from: "2028-04-08T00:00",
            created_to: "2028-04-09T23:59",
            time_zone: zone,
        })
        expect(filter.election_event_id).toBe("event")
        expect(at(String(filter.created_from))).toBe(at(from))
        // The server includes the whole last minute.
        expect(at(String(filter.created_to))).toBe(at(to))
        // The zone is how the bounds are read; it isn't a filter.
        expect(filter).not.toHaveProperty("time_zone")
    })

    it("reads the statement timestamp bounds the same way and drops empty ones", () => {
        const filter = electoralLogInstantFilters({
            statement_timestamp_from: "2028-04-08T09:30:15",
            statement_timestamp_to: "",
            time_zone: "Asia/Kathmandu",
        })
        expect(at(String(filter.statement_timestamp_from))).toBe(at("2028-04-08T03:45:00Z"))
        expect(filter).not.toHaveProperty("statement_timestamp_to")
    })

    it("reads the bounds in the default zone when none is chosen, and sends neither", () => {
        const filter = electoralLogInstantFilters({
            created_from: "2028-04-08T00:00",
            time_zone: "",
            default_time_zone: "Asia/Manila",
        })
        expect(at(String(filter.created_from))).toBe(at("2028-04-07T16:00:00Z"))
        expect(filter).not.toHaveProperty("time_zone")
        expect(filter).not.toHaveProperty("default_time_zone")
    })

    it("reads the bounds in the browser's zone when no zone is chosen", () => {
        const {browserTimeZone, zonedToInstant} = jest.requireActual(
            "../../../ui-core/src/services/timeZones"
        )
        const filter = electoralLogInstantFilters({created_from: "2028-04-08T00:00"})
        expect(filter.created_from).toBe(
            zonedToInstant("2028-04-08T00:00", browserTimeZone()).instant
        )
    })
})
