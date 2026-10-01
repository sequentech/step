// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EDataSource, EDynamicOptions, ESelectorControl, type MonitoringWidget} from "../types"
import {
    ESelectorState,
    dashboardSelectorValues,
    resolveSelectors,
    selectorValuesForRequest,
} from "./selectors"

const widget: MonitoringWidget = {
    id: "turnout-by-group",
    title: "Turnout by group",
    source: EDataSource.VOTER_TURNOUT,
    selectors: {
        breakdown: {
            label: "Breakdown",
            options: {sex: "Sex", age_band: "Age"},
            default: "age_band",
        },
        grain: {
            label: "Grain",
            options: {hour: "Hourly", day: "Daily"},
            default: "day",
            control: ESelectorControl.TOGGLE,
        },
        day: {
            label: "Day",
            options_from: EDynamicOptions.EVENT_DAYS,
            when: {selector: "grain", in: ["hour"]},
        },
    },
    chart: {},
}

describe("resolveSelectors", () => {
    it("takes the viewer's pick, then the dashboard value, then the default", () => {
        const byDefault = resolveSelectors(widget, {}, {}, {})
        expect(byDefault[0].state).toEqual({kind: ESelectorState.VALUE, value: "age_band"})

        const fromDashboard = resolveSelectors(widget, {breakdown: "sex"}, {}, {})
        expect(fromDashboard[0].state).toEqual({kind: ESelectorState.VALUE, value: "sex"})

        const picked = resolveSelectors(widget, {breakdown: "sex"}, {breakdown: "age_band"}, {})
        expect(picked[0].state).toEqual({kind: ESelectorState.VALUE, value: "age_band"})
    })

    it("ignores a pick or dashboard value that is not a listed option", () => {
        const resolved = resolveSelectors(widget, {breakdown: "gone"}, {breakdown: "renamed"}, {})
        expect(resolved[0].state).toEqual({kind: ESelectorState.VALUE, value: "age_band"})
    })

    it("hides a selector whose condition does not hold", () => {
        const resolved = resolveSelectors(widget, {}, {}, {event_days: ["2026-05-01"]})
        expect(resolved.map((selector) => selector.state.kind)).toEqual([
            ESelectorState.VALUE,
            ESelectorState.VALUE,
            ESelectorState.HIDDEN,
        ])
    })

    it("defaults dynamic options to the latest and reports none when there are none", () => {
        const days = {event_days: ["2026-05-01", "2026-05-02"]}
        const hourly = resolveSelectors(widget, {}, {grain: "hour"}, days)
        expect(hourly[2].state).toEqual({kind: ESelectorState.VALUE, value: "2026-05-02"})
        expect(hourly[2].options.map((option) => option.value)).toEqual([
            "2026-05-01",
            "2026-05-02",
        ])
        const noDays = resolveSelectors(widget, {}, {grain: "hour"}, {})
        expect(noDays[2].state).toEqual({kind: ESelectorState.NO_OPTIONS})
    })

    it("sends only the visible selectors that have a value", () => {
        const resolved = resolveSelectors(widget, {}, {grain: "hour"}, {})
        expect(selectorValuesForRequest(resolved)).toEqual({breakdown: "age_band", grain: "hour"})
    })

    it("resolves a widget without selectors to nothing", () => {
        expect(resolveSelectors({...widget, selectors: undefined}, {}, {}, {})).toEqual([])
    })
})

describe("dashboardSelectorValues", () => {
    const cell = (key: string, values: Record<string, string>, shown = widget) => ({
        key,
        widgetId: shown.id,
        values,
        widget: shown,
    })
    const days = {event_days: ["2026-05-01", "2026-05-02"]}

    it("gives each widget the values it is drawn with, by widget id", () => {
        const plain: MonitoringWidget = {...widget, id: "summary", selectors: undefined}
        const cells = [cell("0:turnout-by-group", {breakdown: "sex"}), cell("1:summary", {}, plain)]
        const picks: Record<string, Record<string, string>> = {
            "0:turnout-by-group": {grain: "hour"},
        }
        expect(
            dashboardSelectorValues(cells, (placement) => picks[placement.key] ?? {}, days)
        ).toEqual({
            "turnout-by-group": {breakdown: "sex", grain: "hour", day: "2026-05-02"},
            "summary": {},
        })
    })

    it("takes a widget placed twice as its first placement shows it, and skips a missing one", () => {
        const cells = [
            cell("0:turnout-by-group", {breakdown: "sex"}),
            cell("2:turnout-by-group", {breakdown: "age_band"}),
            {key: "3:gone", widgetId: "gone", values: {}, widget: null},
        ]
        expect(dashboardSelectorValues(cells, () => ({}), days)).toEqual({
            "turnout-by-group": {breakdown: "sex", grain: "day"},
        })
    })
})
