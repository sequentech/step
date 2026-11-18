// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {SUPPORT_REFERENCE_FILTER} from "./supportReference"

/** A filter input of the queue: where it writes its value and, for a choice, what it offers. */
export interface IQueueFilter {
    source: string
    choices?: string[]
}

const COLUMN = /^[a-z][a-z0-9_]*$/
const ORDERS = ["ASC", "DESC"]
// React-admin's default page sizes.
const PAGE_SIZES = [5, 10, 25, 50]

const isObject = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

/** Each value of a (react-admin nested) filter with the input source it came from. */
const leaves = (filter: Record<string, unknown>, path = ""): [string, unknown][] =>
    Object.entries(filter).flatMap(([key, value]) =>
        isObject(value) ? leaves(value, `${path}${key}.`) : [[`${path}${key}`, value]]
    )

/**
 * The list parameters react-admin saved for the queue, if the queue can still
 * use them: every filter is one of its inputs, with a value it offers, so a
 * saved list never chooses the event or the election's label. The applications
 * a support reference was resolved to are dropped; they are looked up again.
 * `undefined` makes the queue start as on a first visit. An unusable sort or
 * page is left to the queue's own, keeping the filters.
 */
export const restorableListParams = (
    saved: unknown,
    filters: IQueueFilter[]
): Record<string, unknown> | undefined => {
    if (!isObject(saved)) return undefined
    const {filter = {}, displayedFilters = {}, sort, order, page, perPage, ...unknown} = saved
    if (Object.keys(unknown).length > 0 || !isObject(filter) || !isObject(displayedFilters)) {
        return undefined
    }
    const {[SUPPORT_REFERENCE_FILTER]: _resolved, ...chosen} = filter
    const inputs = new Map(filters.map((input) => [input.source, input]))
    const usableFilters =
        leaves(chosen).every(([source, value]) => {
            const input = inputs.get(source)
            return (
                input !== undefined &&
                typeof value === "string" &&
                (input.choices?.includes(value) ?? true)
            )
        }) &&
        Object.entries(displayedFilters).every(
            ([source, shown]) => inputs.has(source) && typeof shown === "boolean"
        )
    if (!usableFilters) return undefined
    const restored: Record<string, unknown> = {...saved, filter: chosen}
    // Any one column can be restored: the data provider ignores a sort by one it doesn't know.
    if (typeof sort !== "string" || !COLUMN.test(sort) || !ORDERS.includes(order as string)) {
        delete restored.sort
        delete restored.order
    }
    if (!Number.isInteger(page) || (page as number) < 1) delete restored.page
    if (!PAGE_SIZES.includes(perPage as number)) delete restored.perPage
    return restored
}
