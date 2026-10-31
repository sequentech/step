// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Reading the parts of a widget definition the forms show. The YAML is the
 * source of truth, so these only interpret `draft.value`; writing goes
 * through `yamlPatch`.
 */

import type {IMonitoringLayoutItem, IMonitoringWidgetDefinition} from "./types"
import {EMonitoringScopeSelector} from "./types"

/** `{selector: breakdown}` → `"breakdown"`; anything else is a literal. */
export const selectorRef = (value: unknown): string | undefined => {
    if (!value || typeof value !== "object" || Array.isArray(value)) return undefined
    const entries = Object.entries(value as Record<string, unknown>)
    return entries.length === 1 && entries[0][0] === "selector" && typeof entries[0][1] === "string"
        ? entries[0][1]
        : undefined
}

export const asWidget = (value: unknown): IMonitoringWidgetDefinition =>
    value && typeof value === "object" && !Array.isArray(value)
        ? (value as IMonitoringWidgetDefinition)
        : {}

export const stringList = (value: unknown): string[] =>
    Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : []

export const SCOPE_SELECTORS = Object.values(EMonitoringScopeSelector)

/** Which dashboard selectors a widget follows: absent `follows` means every one. */
export const followedSelectors = (
    widget: IMonitoringWidgetDefinition
): EMonitoringScopeSelector[] =>
    widget.follows === undefined
        ? SCOPE_SELECTORS
        : SCOPE_SELECTORS.filter((selector) => widget.follows?.includes(selector))

/** The `follows` list after ticking or unticking one selector, `undefined` for all of them. */
export const toggleFollowed = (
    widget: IMonitoringWidgetDefinition,
    selector: EMonitoringScopeSelector,
    followed: boolean
): string[] | undefined => {
    const current = new Set<string>(followedSelectors(widget))
    if (followed) current.add(selector)
    else current.delete(selector)
    const next = SCOPE_SELECTORS.filter((item) => current.has(item))
    return next.length === SCOPE_SELECTORS.length ? undefined : next
}

/** `turnout-by-group` → the first free `turnout-by-group-copy`, `-copy-2`, … */
export const copyId = (id: string, taken: ReadonlySet<string>): string => {
    const base = `${id}-copy`
    if (!taken.has(base)) return base
    let index = 2
    while (taken.has(`${base}-${index}`)) index += 1
    return `${base}-${index}`
}

/** A name not yet used in `names`: `selector_1`, `selector_2`, … */
export const freshName = (prefix: string, names: ReadonlyArray<string>): string => {
    let index = 1
    while (names.includes(`${prefix}_${index}`)) index += 1
    return `${prefix}_${index}`
}

/** `pre_enrolled` → `Pre enrolled`, for ids the platform has no words for. */
export const humanize = (id: string): string => {
    const words = id.replace(/[_-]+/g, " ").trim()
    return words ? words[0].toUpperCase() + words.slice(1) : id
}

/** A layout item and its position in the YAML's `layout` sequence. */
export interface ILayoutEntry {
    item: IMonitoringLayoutItem
    /** Index into the raw `layout`, which every patch addresses. */
    index: number
}

/**
 * The dashboard's layout items the form can show, each with its index in
 * the YAML. `malformed` when some item is not `{widget, width}`: the form
 * leaves it out, so its neighbours' positions no longer follow one another.
 */
export const layoutEntries = (value: unknown): {entries: ILayoutEntry[]; malformed: boolean} => {
    const layout =
        value && typeof value === "object" && !Array.isArray(value)
            ? (value as {layout?: unknown}).layout
            : undefined
    if (!Array.isArray(layout)) return {entries: [], malformed: layout !== undefined}
    const entries = layout.flatMap((item: unknown, index): ILayoutEntry[] =>
        item &&
        typeof item === "object" &&
        typeof (item as IMonitoringLayoutItem).widget === "string"
            ? [{item: item as IMonitoringLayoutItem, index}]
            : []
    )
    return {entries, malformed: entries.length !== layout.length}
}
