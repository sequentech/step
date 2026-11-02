// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EDataSource, type MonitoringDashboard, type MonitoringWidget} from "../types"
import {clampWidth, gridItemSize, layoutCells} from "./layout"

const widget = (id: string): MonitoringWidget => ({
    id,
    title: id,
    source: EDataSource.VOTER_TURNOUT,
    chart: {},
})

describe("layout", () => {
    it("keeps widths on the 12-column grid", () => {
        expect(clampWidth(6)).toBe(6)
        expect(clampWidth(13)).toBe(12)
        expect(clampWidth(0)).toBe(1)
        expect(clampWidth(Number.NaN)).toBe(12)
        expect(clampWidth(4.6)).toBe(5)
    })

    it("fills the row on small screens and uses the width from md up", () => {
        expect(gridItemSize(4)).toEqual({xs: 12, md: 4})
    })

    it("lists cells in layout order, marking widgets that are missing or invalid", () => {
        const dashboard: MonitoringDashboard = {
            id: "d",
            title: "D",
            layout: [
                {widget: "a", width: 12},
                {widget: "missing", width: 6},
                {widget: "a", width: 6, values: {measure: "voted_pre"}},
                {widget: "broken", width: 6},
            ],
        }
        const cells = layoutCells(dashboard, {
            a: {widget: widget("a"), revision: 3},
            broken: {widget: null, revision: 1, problem: "bad"},
        })
        expect(cells.map((cell) => [cell.key, cell.widget?.id ?? null, cell.problem])).toEqual([
            ["0:a", "a", undefined],
            ["1:missing", null, "missing"],
            ["2:a", "a", undefined],
            ["3:broken", null, "bad"],
        ])
        expect(cells[2].values).toEqual({measure: "voted_pre"})
        expect(cells[0].revision).toBe(3)
    })
})
