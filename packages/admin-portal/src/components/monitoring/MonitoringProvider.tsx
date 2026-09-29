// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {
    createContext,
    useContext,
    useEffect,
    useMemo,
    useReducer,
    type PropsWithChildren,
} from "react"
import {
    EMonitoringAction,
    INITIAL_STATE,
    loadState,
    monitoringReducer,
    placementValues,
    saveState,
    setPlacementValue,
    type WidgetPlacement,
} from "./lib/state"
import type {EMonitoringViewMode, MonitoringScope, MonitoringState} from "./types"

/**
 * What the editor adds to the view. Each action appears in the dashboard only
 * when given, so a viewer without the configure permission sees none.
 */
export interface MonitoringEditorActions {
    onConfigureWidget?: (widgetId: string) => void
    onDuplicateWidget?: (widgetId: string) => void
    onEditDashboard?: (dashboardId: string) => void
}

interface MonitoringContextValue {
    state: MonitoringState
    selectDashboard: (dashboardId: string) => void
    setScope: (scope: MonitoringScope) => void
    /** A pick for one placement of a widget: one placed twice keeps two sets. */
    setWidgetValue: (placement: WidgetPlacement, name: string, value: string | null) => void
    widgetValues: (placement: WidgetPlacement) => Record<string, string>
    setMode: (mode: EMonitoringViewMode) => void
    actions: MonitoringEditorActions
}

const noop = () => undefined
const NO_ACTIONS: MonitoringEditorActions = {}
const NO_VALUES: Record<string, string> = {}

const MonitoringContext = createContext<MonitoringContextValue>({
    state: INITIAL_STATE,
    selectDashboard: noop,
    setScope: noop,
    setWidgetValue: noop,
    widgetValues: () => NO_VALUES,
    setMode: noop,
    actions: NO_ACTIONS,
})

const sessionStorageOrNothing = (): Storage | undefined => {
    try {
        return window.sessionStorage
    } catch {
        return undefined
    }
}

export function MonitoringProvider({
    storageKey,
    actions = NO_ACTIONS,
    children,
}: PropsWithChildren<{
    /** One per event and election page, so each keeps its own choice. */
    storageKey: string
    actions?: MonitoringEditorActions
}>) {
    const [state, dispatch] = useReducer(monitoringReducer, storageKey, (key) =>
        loadState(sessionStorageOrNothing(), key)
    )
    useEffect(() => saveState(sessionStorageOrNothing(), storageKey, state), [storageKey, state])

    const value = useMemo<MonitoringContextValue>(() => {
        const dashboardId = state.dashboardId ?? ""
        return {
            state,
            selectDashboard: (id) =>
                dispatch({type: EMonitoringAction.SELECT_DASHBOARD, dashboardId: id}),
            setScope: (scope) => dispatch({type: EMonitoringAction.SET_SCOPE, scope}),
            setWidgetValue: (placement, name, selected) =>
                dispatch(setPlacementValue(dashboardId, placement, name, selected)),
            widgetValues: (placement) => placementValues(state, dashboardId, placement),
            setMode: (mode) => dispatch({type: EMonitoringAction.SET_MODE, mode}),
            actions,
        }
    }, [state, actions])

    return <MonitoringContext.Provider value={value}>{children}</MonitoringContext.Provider>
}

export const useMonitoring = () => useContext(MonitoringContext)
