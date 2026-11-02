// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMonitoringViewMode} from "../types"
import {
    EMonitoringAction,
    INITIAL_STATE,
    loadState,
    monitoringReducer,
    placementValues,
    saveState,
    setPlacementValue,
    widgetValueKey,
} from "./state"

class MemoryStorage {
    items = new Map<string, string>()
    getItem = (key: string) => this.items.get(key) ?? null
    setItem = (key: string, value: string) => void this.items.set(key, value)
}

describe("monitoringReducer", () => {
    it("selects a dashboard, keeping the scope", () => {
        const scoped = monitoringReducer(INITIAL_STATE, {
            type: EMonitoringAction.SET_SCOPE,
            scope: {region: "north"},
        })
        const next = monitoringReducer(scoped, {
            type: EMonitoringAction.SELECT_DASHBOARD,
            dashboardId: "req-0260",
        })
        expect(next.dashboardId).toBe("req-0260")
        expect(next.dashboardValues).toEqual({region: "north"})
    })

    it("keeps a widget's picks per dashboard, and clears one with null", () => {
        const key = widgetValueKey("overview", "turnout")
        const picked = monitoringReducer(INITIAL_STATE, {
            type: EMonitoringAction.SET_WIDGET_VALUE,
            key,
            name: "measure",
            value: "voted_pre",
        })
        expect(picked.widgetValues).toEqual({"overview/turnout": {measure: "voted_pre"}})
        const cleared = monitoringReducer(picked, {
            type: EMonitoringAction.SET_WIDGET_VALUE,
            key,
            name: "measure",
            value: null,
        })
        expect(cleared.widgetValues).toEqual({"overview/turnout": {}})
    })

    it("keeps the picks of each placement of a widget apart", () => {
        const first = {key: "0:turnout", widgetId: "turnout"}
        const second = {key: "3:turnout", widgetId: "turnout"}
        let state = monitoringReducer(
            INITIAL_STATE,
            setPlacementValue("overview", first, "measure", "voted_pre")
        )
        state = monitoringReducer(
            state,
            setPlacementValue("overview", second, "measure", "pre_reg")
        )
        expect(placementValues(state, "overview", first)).toEqual({measure: "voted_pre"})
        expect(placementValues(state, "overview", second)).toEqual({measure: "pre_reg"})
        expect(placementValues(state, "req-0260", first)).toEqual({})
    })

    it("reads the picks an earlier portal kept by widget id, until a placement has its own", () => {
        const first = {key: "0:turnout", widgetId: "turnout"}
        const second = {key: "3:turnout", widgetId: "turnout"}
        const saved = {
            ...INITIAL_STATE,
            widgetValues: {"overview/turnout": {measure: "voted_pre", breakdown: "sex"}},
        }
        expect(placementValues(saved, "overview", first)).toEqual({
            measure: "voted_pre",
            breakdown: "sex",
        })
        const state = monitoringReducer(
            saved,
            setPlacementValue("overview", first, "measure", null)
        )
        expect(placementValues(state, "overview", first)).toEqual({breakdown: "sex"})
        expect(placementValues(state, "overview", second)).toEqual({
            measure: "voted_pre",
            breakdown: "sex",
        })
    })

    it("switches between viewing and editing", () => {
        const next = monitoringReducer(INITIAL_STATE, {
            type: EMonitoringAction.SET_MODE,
            mode: EMonitoringViewMode.EDIT,
        })
        expect(next.mode).toBe(EMonitoringViewMode.EDIT)
    })
})

describe("session storage", () => {
    it("restores what was saved, but always opens in view mode", () => {
        const storage = new MemoryStorage()
        saveState(storage, "k", {
            ...INITIAL_STATE,
            dashboardId: "overview",
            dashboardValues: {country: "ES"},
            mode: EMonitoringViewMode.EDIT,
        })
        expect(loadState(storage, "k")).toEqual({
            ...INITIAL_STATE,
            dashboardId: "overview",
            dashboardValues: {country: "ES"},
        })
    })

    it("restores picks saved by widget id as well as by placement", () => {
        const storage = new MemoryStorage()
        const widgetValues = {
            "overview/turnout": {measure: "voted_pre"},
            "overview/3:turnout": {measure: "pre_reg"},
        }
        storage.setItem(
            "k",
            JSON.stringify({dashboardId: "overview", dashboardValues: {}, widgetValues})
        )
        expect(loadState(storage, "k").widgetValues).toEqual(widgetValues)
    })

    it("starts afresh from nothing, from bad JSON or from an unexpected shape", () => {
        const storage = new MemoryStorage()
        expect(loadState(storage, "k")).toEqual(INITIAL_STATE)
        storage.setItem("k", "{")
        expect(loadState(storage, "k")).toEqual(INITIAL_STATE)
        storage.setItem("k", JSON.stringify({dashboardId: 3, dashboardValues: {region: 1}}))
        expect(loadState(storage, "k")).toEqual(INITIAL_STATE)
        expect(loadState(undefined, "k")).toEqual(INITIAL_STATE)
    })

    it("keeps working when storage refuses to write", () => {
        const storage = {
            getItem: () => null,
            setItem: () => {
                throw new Error("quota")
            },
        }
        expect(() => saveState(storage, "k", INITIAL_STATE)).not.toThrow()
    })
})
