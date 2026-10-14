// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EDataSource, type MonitoringWidget} from "../types"
import {catalogGroups, matchesSearch} from "./catalog"

const widget = (
    id: string,
    source: string,
    title: string,
    requirements: string[] = []
): MonitoringWidget => ({id, source, title, requirements, chart: {}})

const widgets = [
    widget("poll", EDataSource.POLL_STATUS, "Poll status", ["SW-F-0256"]),
    widget("turnout-by-post", EDataSource.VOTER_TURNOUT, "Turnout by Post", ["SW-F-0261"]),
    widget("turnout", EDataSource.VOTER_TURNOUT, "Voter turnout", ["SW-F-0259", "SW-F-0260"]),
    widget("custom", "a_new_source", "Custom"),
]

describe("catalogGroups", () => {
    it("groups widgets by data source in the platform's order, sorted by title", () => {
        const groups = catalogGroups(widgets, "")
        expect(groups.map((group) => group.source)).toEqual([
            EDataSource.VOTER_TURNOUT,
            EDataSource.POLL_STATUS,
            "a_new_source",
        ])
        expect(groups[0].widgets.map((w) => w.id)).toEqual(["turnout-by-post", "turnout"])
    })

    it("finds a widget by requirement ID, however it is typed", () => {
        expect(catalogGroups(widgets, "0260").flatMap((g) => g.widgets.map((w) => w.id))).toEqual([
            "turnout",
        ])
        expect(matchesSearch(widgets[2], "sw f 0259")).toBe(true)
    })

    it("finds widgets by title or by data source label", () => {
        const label = (source: string) =>
            source === EDataSource.POLL_STATUS ? "Poll status" : source
        expect(catalogGroups(widgets, "POLL", label)[0].widgets[0].id).toBe("poll")
        expect(catalogGroups(widgets, "nothing like it")).toEqual([])
    })
})
