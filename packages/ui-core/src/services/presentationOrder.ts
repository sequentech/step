// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {CandidatesOrder} from "../types/ContestPresentation"
import {ContestsOrder} from "../types/ElectionPresentation"
import {ElectionsOrder} from "../types/ElectionEventPresentation"

export type PresentationOrder = ElectionsOrder | ContestsOrder | CandidatesOrder

interface PresentationWithSortOrder {
    sort_order?: number | null
}

interface PresentationOrderAccessors<T> {
    getLabel: (item: T) => string | null | undefined
    getPresentation: (item: T) => unknown
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

export const parseEntityPresentation = <T extends object>(value: unknown): T | undefined => {
    let parsed = value

    if (typeof value === "string") {
        try {
            parsed = JSON.parse(value) as unknown
        } catch {
            return undefined
        }
    }

    return isRecord(parsed) ? (parsed as T) : undefined
}

const compareText = (left: string, right: string): number => {
    if (left < right) return -1
    if (left > right) return 1
    return 0
}

const sortOrder = (presentation: unknown): number => {
    const value = parseEntityPresentation<PresentationWithSortOrder>(presentation)?.sort_order
    return typeof value === "number" && Number.isFinite(value) ? value : -1
}

/**
 * Applies the configured entity ordering without mutating the source collection.
 * Random order is intentionally preserved rather than regenerated in result views.
 */
export const sortByPresentationOrder = <T>(
    items: readonly T[],
    order: PresentationOrder | null | undefined,
    accessors: PresentationOrderAccessors<T>
): T[] => {
    const sorted = [...items]

    if (order === ElectionsOrder.RANDOM) {
        return sorted
    }

    if (order === ElectionsOrder.CUSTOM) {
        return sorted.sort(
            (left, right) =>
                sortOrder(accessors.getPresentation(left)) -
                sortOrder(accessors.getPresentation(right))
        )
    }

    return sorted.sort((left, right) => {
        const leftLabel = (accessors.getLabel(left) ?? "").toLowerCase()
        const rightLabel = (accessors.getLabel(right) ?? "").toLowerCase()
        return compareText(leftLabel, rightLabel)
    })
}
