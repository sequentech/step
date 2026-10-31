// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EColumnKind, type MonitoringTable} from "../types"
import {columnLabeler, dataSections, widgetQueries} from "./dataTables"

const totals: MonitoringTable = {
    columns: [
        {name: "registered", kind: EColumnKind.INTEGER},
        {name: "pre_enrolled", kind: EColumnKind.INTEGER},
        {name: "voted", kind: EColumnKind.INTEGER},
    ],
    rows: [[8, 0, 3]],
}
const votedReg: MonitoringTable = {
    columns: [
        {name: "numerator", kind: EColumnKind.INTEGER},
        {name: "denominator", kind: EColumnKind.INTEGER},
        {name: "pct", kind: EColumnKind.NUMBER},
        {name: "pct_label", kind: EColumnKind.TEXT},
    ],
    rows: [[3, 8, 0.375, "37.5%"]],
}

// Turnout summary as the preset defines it.
const queries = {
    totals: {template: "summary", measures: ["registered", "pre_enrolled", "voted"]},
    voted_reg: {template: "summary", ratio: ["voted", "registered"]},
    voted_pre: {template: "summary", ratio: ["voted", "pre_enrolled"]},
}

const WORDS: Record<string, string> = {
    "monitoring.measures.registered": "Registered",
    "monitoring.measures.pre_enrolled": "Pre-enrolled",
    "monitoring.measures.voted": "Voted",
    "monitoring.columns.numerator": "Numerator",
    "monitoring.columns.pct": "Percentage",
    "monitoring.columns.pct_label": "Percentage as shown",
    "monitoring.columns.group": "Group",
    "monitoring.columns.region": "Region",
}
const known = (key: string): string | undefined => WORDS[key]

describe("dataSections", () => {
    it("has a section per query, in widget order", () => {
        const sections = dataSections(
            {
                table: totals,
                tables: [
                    {query: "totals", table: totals},
                    {query: "voted_reg", table: votedReg},
                ],
            },
            queries
        )
        expect(sections.map((section) => section.query)).toEqual(["totals", "voted_reg"])
        expect(sections[1].table).toBe(votedReg)
        expect(sections[1].definition).toBe(queries.voted_reg)
    })

    it("leaves out a query with no rows to show", () => {
        const sections = dataSections(
            {
                tables: [
                    {query: "totals", table: totals},
                    {query: "voted_reg", table: null},
                ],
            },
            queries
        )
        expect(sections.map((section) => section.query)).toEqual(["totals"])
    })

    it("shows the one table of an older backend, untitled, as the first query", () => {
        const [section, ...rest] = dataSections({table: totals}, queries)
        expect(rest).toEqual([])
        expect(section).toEqual({query: null, table: totals, definition: queries.totals})
        expect(dataSections({table: totals, tables: []}, queries)).toHaveLength(1)
    })

    it("has nothing to show without a table", () => {
        expect(dataSections({table: null}, queries)).toEqual([])
        expect(dataSections({}, undefined)).toEqual([])
    })
})

describe("columnLabeler", () => {
    it("names measure columns with the platform's words", () => {
        const label = columnLabeler(queries.totals, known)
        expect(totals.columns.map((column) => label(column.name))).toEqual([
            "Registered",
            "Pre-enrolled",
            "Voted",
        ])
    })

    it("names a ratio's numerator and denominator by their measures", () => {
        const label = columnLabeler(queries.voted_reg, known)
        expect(votedReg.columns.map((column) => label(column.name))).toEqual([
            "Voted",
            "Registered",
            "Percentage",
            "Percentage as shown",
        ])
    })

    it("uses the query's own labels first", () => {
        const label = columnLabeler(
            {template: "summary", ratio: ["voted", "registered"], labels: {voted: "Ballots"}},
            known
        )
        expect(label("numerator")).toBe("Ballots")
        expect(label("voted")).toBe("Ballots")
        expect(label("denominator")).toBe("Registered")
    })

    it("names the group column by the dimension grouped by", () => {
        expect(columnLabeler({template: "by_group", group_by: "region"}, known)("group")).toBe(
            "Region"
        )
        expect(columnLabeler({template: "by_group", group_by: "sex"}, known)("group")).toBe("sex")
        // A selector picks the dimension: the generic word.
        expect(
            columnLabeler({template: "by_group", group_by: {selector: "breakdown"}}, known)("group")
        ).toBe("Group")
    })

    it("keeps the raw name when no word is known", () => {
        const label = columnLabeler(undefined, known)
        expect(label("denominator")).toBe("denominator")
        expect(label("bucket_utc")).toBe("bucket_utc")
        expect(label("numerator")).toBe("Numerator")
    })
})

describe("widgetQueries", () => {
    it("names a widget's one query data, as its charts read it", () => {
        const query = {template: "summary", measures: ["voted"]}
        expect(widgetQueries({query})).toEqual({data: query})
        expect(widgetQueries({queries})).toBe(queries)
        expect(widgetQueries({queries: {}, query})).toEqual({data: query})
        expect(widgetQueries(undefined)).toBeUndefined()
    })
})
