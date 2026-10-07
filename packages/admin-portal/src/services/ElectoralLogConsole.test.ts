// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    columnField,
    ConsoleTenant,
    displayValue,
    eventElections,
    invalidFilters,
    isQueryError,
    ROW_ID,
    toConsoleFilters,
    tenantEvents,
    toGridRows,
} from "./ElectoralLogConsole"

const ELECTION = "0499c208-1c9b-4a49-9a51-0d8f0bba2f5e"

describe("electoral-log console filters", () => {
    it("keep only the table's filled filters, with dates in seconds", () => {
        const values = {
            statement_kind: " CastVote ",
            area_id: "area",
            status: "",
            created_after: "2026-10-05T20:00:00Z",
        }
        expect(toConsoleFilters("records", values)).toEqual({
            statement_kind: "CastVote",
            created_after: 1791230400,
        })
        expect(toConsoleFilters("queue", values)).toEqual({area_id: "area"})
        expect(toConsoleFilters("voters", {})).toEqual({})
    })

    it("refuse dates that do not parse and ballot-box IDs that are not UUIDs", () => {
        expect(
            invalidFilters("ballots", {
                election_id: ELECTION,
                area_id: "north",
                created_before: "yesterday",
                user_id: "anything",
            })
        ).toEqual(["area_id", "created_before"])
        expect(invalidFilters("records", {election_id: "not-a-uuid"})).toEqual([])
        expect(invalidFilters("queue", {created_before: "yesterday"})).toEqual([])
    })
})

describe("electoral-log console rows", () => {
    it("become grid rows keyed by column position", () => {
        const rows = toGridRows(
            {
                columns: ["id", "id", "payload"],
                rows: [
                    [1, 2, {a: 1}],
                    [3, null],
                ],
            },
            25
        )
        expect(rows).toEqual([
            {[ROW_ID]: 25, c0: 1, c1: 2, c2: {a: 1}},
            {[ROW_ID]: 26, c0: 3, c1: null, c2: null},
        ])
        expect(columnField(2)).toBe("c2")
    })

    it("show values as text", () => {
        expect(displayValue(null)).toBe("")
        expect(displayValue({a: [1]})).toBe('{"a":[1]}')
        expect(displayValue(false)).toBe("false")
        expect(displayValue(12)).toBe("12")
    })

    it("tell query errors from results", () => {
        expect(isQueryError({error: "syntax error"})).toBe(true)
        expect(isQueryError({columns: [], rows: [], truncated: false, elapsed_ms: 3})).toBe(false)
    })
})

describe("electoral-log console tenants", () => {
    const tenants: ConsoleTenant[] = [
        {
            id: "acme",
            slug: "acme",
            events: [
                {id: "spring", is_archived: false, elections: [{id: "mayor"}, {id: "council"}]},
                {id: "autumn", is_archived: true, elections: []},
            ],
        },
        {id: "empty", slug: "empty", events: []},
    ]

    it("list a tenant's election events", () => {
        expect(tenantEvents(tenants, "acme").map((event) => event.id)).toEqual(["spring", "autumn"])
        expect(tenantEvents(tenants, "empty")).toEqual([])
        expect(tenantEvents(tenants, "unknown")).toEqual([])
    })

    it("list an election event's elections", () => {
        expect(eventElections(tenants, "acme", "spring").map((election) => election.id)).toEqual([
            "mayor",
            "council",
        ])
        expect(eventElections(tenants, "acme", "autumn")).toEqual([])
        expect(eventElections(tenants, "empty", "spring")).toEqual([])
    })
})
