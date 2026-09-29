// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMonitoringViewMode} from "../types"
import {
    EMonitoringAction,
    INITIAL_STATE,
    loadState,
    monitoringReducer,
    saveState,
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
