// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {EDynamicOptions, MonitoringSelector, MonitoringWidget} from "../types"
import {conditionHolds} from "./when"

// Widget selector values as `sequent_core::monitoring::resolve` settles them:
// the viewer's pick, then the dashboard's value for the widget, then the
// selector's default. A value that is not an option — a pick kept in the
// session from an earlier revision — falls through instead of being sent.

export enum ESelectorState {
    VALUE = "VALUE",
    /** Its `when` condition does not hold: not shown, feeds nothing. */
    HIDDEN = "HIDDEN",
    /** Its options come from the data, and there are none yet. */
    NO_OPTIONS = "NO_OPTIONS",
}

export type SelectorState =
    | {kind: ESelectorState.VALUE; value: string}
    | {kind: ESelectorState.HIDDEN}
    | {kind: ESelectorState.NO_OPTIONS}

export const value = (text: string): SelectorState => ({kind: ESelectorState.VALUE, value: text})
export const hidden = (): SelectorState => ({kind: ESelectorState.HIDDEN})
const noOptions = (): SelectorState => ({kind: ESelectorState.NO_OPTIONS})

export interface SelectorOption {
    value: string
    label: string
}

export interface ResolvedSelector {
    name: string
    selector: MonitoringSelector
    options: SelectorOption[]
    state: SelectorState
}

export type DynamicOptionValues = Partial<Record<EDynamicOptions, string[]>>

export function selectorOptions(
    selector: MonitoringSelector,
    dynamic: DynamicOptionValues
): SelectorOption[] {
    if (selector.options_from) {
        return (dynamic[selector.options_from] ?? []).map((option) => ({
            value: option,
            label: option,
        }))
    }
    return Object.entries(selector.options ?? {}).map(([option, label]) => ({
        value: option,
        label,
    }))
}

export function resolveSelectors(
    widget: MonitoringWidget,
    dashboardValues: Record<string, string>,
    requested: Record<string, string>,
    dynamic: DynamicOptionValues
): ResolvedSelector[] {
    const states: Record<string, SelectorState> = {}
    return Object.entries(widget.selectors ?? {}).map(([name, selector]) => {
        const options = selectorOptions(selector, dynamic)
        const state = selectorState(
            selector,
            options,
            states,
            requested[name],
            dashboardValues[name]
        )
        states[name] = state
        return {name, selector, options, state}
    })
}

function selectorState(
    selector: MonitoringSelector,
    options: SelectorOption[],
    earlier: Record<string, SelectorState>,
    requested: string | undefined,
    dashboard: string | undefined
): SelectorState {
    if (!conditionHolds(selector.when, earlier)) return hidden()
    const offered = (candidate: string | undefined): candidate is string =>
        candidate !== undefined && options.some((option) => option.value === candidate)
    if (offered(requested)) return value(requested)
    if (offered(dashboard)) return value(dashboard)
    const fallback = selector.options_from
        ? options.at(-1)?.value
        : offered(selector.default)
          ? selector.default
          : options[0]?.value
    return fallback === undefined ? noOptions() : value(fallback)
}

/** The values a render or export request carries: the shown selectors that have one. */
export function selectorValuesForRequest(resolved: ResolvedSelector[]): Record<string, string> {
    return Object.fromEntries(
        resolved.flatMap(({name, state}) =>
            state.kind === ESelectorState.VALUE ? [[name, state.value]] : []
        )
    )
}

/** A layout cell, as far as its selector values go. */
export interface SelectorPlacement {
    key: string
    widgetId: string
    values: Record<string, string>
    widget: MonitoringWidget | null
}

/**
 * A dashboard export's `widget_selector_values`: each widget's values as the
 * dashboard draws it, by widget id. A widget placed twice is exported as its
 * first placement shows it.
 */
export function dashboardSelectorValues(
    cells: SelectorPlacement[],
    picksOf: (cell: SelectorPlacement) => Record<string, string>,
    dynamic: DynamicOptionValues
): Record<string, Record<string, string>> {
    const values: Record<string, Record<string, string>> = {}
    for (const cell of cells) {
        if (!cell.widget || cell.widgetId in values) continue
        values[cell.widgetId] = selectorValuesForRequest(
            resolveSelectors(cell.widget, cell.values, picksOf(cell), dynamic)
        )
    }
    return values
}
