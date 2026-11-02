// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {parseDashboard, parseWidget, parseWidgets} from "./parseDefinitions"

const widget = {
    id: "turnout",
    title: "Voter turnout",
    source: "voter_turnout",
    selectors: {grain: {label: "Grain", options: {hour: "Hourly"}, default: "hour"}},
    chart: {charts: {}},
    height: 320,
}

describe("parseDefinitions", () => {
    it("accepts a dashboard and a widget, as objects or as JSON text", () => {
        const dashboard = {
            id: "overview",
            title: "Overview",
            layout: [{widget: "turnout", width: 6}],
        }
        expect(parseDashboard(dashboard)).toEqual({ok: true, value: dashboard})
        expect(parseDashboard(JSON.stringify(dashboard))).toEqual({ok: true, value: dashboard})
        expect(parseWidget(widget)).toEqual({ok: true, value: widget})
    })

    it("refuses what cannot be drawn, saying why", () => {
        expect(parseDashboard(null).ok).toBe(false)
        expect(parseDashboard("{not json").ok).toBe(false)
        expect(parseDashboard({id: "x", title: "X", layout: [{widget: 3, width: 6}]})).toEqual({
            ok: false,
            problem: "layout[0].widget is not text",
        })
        expect(parseWidget({...widget, title: undefined})).toEqual({
            ok: false,
            problem: "title is not text",
        })
        expect(parseWidget({...widget, selectors: {grain: {options: {}}}})).toEqual({
            ok: false,
            problem: "selectors.grain.label is not text",
        })
        expect(parseWidget({...widget, height: "tall"}).ok).toBe(false)
    })

    it("parses each widget on its own, so one bad widget does not hide the rest", () => {
        const parsed = parseWidgets({
            turnout: {definition: widget, revision: 2},
            broken: {definition: {id: "broken"}, revision: 1},
        })
        expect(parsed.turnout).toEqual({widget, revision: 2})
        expect(parsed.broken.widget).toBeNull()
        expect(parsed.broken.problem).toBe("title is not text")
        expect(parseWidgets(undefined)).toEqual({})
    })
})
