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

describe("electoral-log console filters", () => {
    it("keep only the filled filters, with dates in seconds", () => {
        const values = {
            statement_kind: " CastVote ",
            user_id: "",
            created_after: "2026-10-05T20:00:00Z",
        }
        expect(toConsoleFilters("records", values)).toEqual({
            statement_kind: "CastVote",
            created_after: 1791230400,
        })
        expect(toConsoleFilters("records", {})).toEqual({})
    })

    it("refuse dates that do not parse", () => {
        expect(
            invalidFilters("records", {
                election_id: "not-a-uuid",
                created_before: "yesterday",
                user_id: "anything",
            })
        ).toEqual(["created_before"])
        expect(invalidFilters("records", {created_after: "2026-10-05T20:00:00Z"})).toEqual([])
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
