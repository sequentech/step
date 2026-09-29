// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EMonitoringViewMode,
    SCOPE_SELECTORS,
    type MonitoringScope,
    type MonitoringState,
} from "../types"

// What the viewer chose on the Dashboard tab. The event and election tabs
// rewrite the URL, so the choice is kept in sessionStorage instead of the
// query string.

export enum EMonitoringAction {
    SELECT_DASHBOARD = "SELECT_DASHBOARD",
    SET_SCOPE = "SET_SCOPE",
    SET_WIDGET_VALUE = "SET_WIDGET_VALUE",
    SET_MODE = "SET_MODE",
}

export type MonitoringActionMessage =
    | {type: EMonitoringAction.SELECT_DASHBOARD; dashboardId: string}
    | {type: EMonitoringAction.SET_SCOPE; scope: MonitoringScope}
    | {type: EMonitoringAction.SET_WIDGET_VALUE; key: string; name: string; value: string | null}
    | {type: EMonitoringAction.SET_MODE; mode: EMonitoringViewMode}

export const INITIAL_STATE: MonitoringState = {
    dashboardId: null,
    dashboardValues: {},
    widgetValues: {},
    mode: EMonitoringViewMode.VIEW,
}

/** A dashboard may give a widget other defaults, so picks are kept per dashboard. */
export const widgetValueKey = (dashboardId: string, widgetId: string) =>
    `${dashboardId}/${widgetId}`

export function monitoringReducer(
    state: MonitoringState,
    action: MonitoringActionMessage
): MonitoringState {
    switch (action.type) {
        case EMonitoringAction.SELECT_DASHBOARD:
            return {...state, dashboardId: action.dashboardId}
        case EMonitoringAction.SET_SCOPE:
            return {...state, dashboardValues: action.scope}
        case EMonitoringAction.SET_WIDGET_VALUE: {
            const values = {...(state.widgetValues[action.key] ?? {})}
            if (action.value === null) delete values[action.name]
            else values[action.name] = action.value
            return {...state, widgetValues: {...state.widgetValues, [action.key]: values}}
        }
        case EMonitoringAction.SET_MODE:
            return {...state, mode: action.mode}
    }
}

export type StateStorage = Pick<Storage, "getItem" | "setItem">

const isTextMap = (value: unknown): value is Record<string, string> =>
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    Object.values(value).every((item) => typeof item === "string")

function isScope(value: unknown): value is MonitoringScope {
    return (
        isTextMap(value) &&
        Object.keys(value).every((key) => SCOPE_SELECTORS.includes(key as never))
    )
}

/** The saved choice, or a fresh one; editing is never resumed from storage. */
export function loadState(storage: StateStorage | undefined, key: string): MonitoringState {
    try {
        const saved: unknown = JSON.parse(storage?.getItem(key) ?? "null")
        if (typeof saved !== "object" || saved === null) return INITIAL_STATE
        const {dashboardId, dashboardValues, widgetValues} = saved as Record<string, unknown>
        if (dashboardId !== null && typeof dashboardId !== "string") return INITIAL_STATE
        if (!isScope(dashboardValues)) return INITIAL_STATE
        if (
            typeof widgetValues !== "object" ||
            widgetValues === null ||
            !Object.values(widgetValues).every(isTextMap)
        ) {
            return INITIAL_STATE
        }
        return {
            dashboardId,
            dashboardValues,
            widgetValues: widgetValues as MonitoringState["widgetValues"],
            mode: EMonitoringViewMode.VIEW,
        }
    } catch {
        return INITIAL_STATE
    }
}

export function saveState(storage: StateStorage | undefined, key: string, state: MonitoringState) {
    try {
        const {dashboardId, dashboardValues, widgetValues} = state
        storage?.setItem(key, JSON.stringify({dashboardId, dashboardValues, widgetValues}))
    } catch {
        // A full or disabled storage only means the choice is not remembered.
    }
}
