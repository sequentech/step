// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const convertToNumber = <T>(val: T) => {
    if (val === null || val === undefined) {
        return null
    }

    if (typeof val === "number") {
        return val
    }

    if (typeof val === "boolean") {
        return val ? 1 : 0
    }

    if (typeof val === "string") {
        const num = parseFloat(val)
        return isNaN(num) ? null : num
    }

    // For any other types return null
    return null
}

export const getPreferenceKey = (key: string, subkey: string) => {
    return `${key.replaceAll("/", "_").replaceAll("-", "_")}_${subkey}`
}

// function to convert null values to void string for comparison
const customSortComparator = (
    a: Record<string, any>,
    b: Record<string, any>,
    field: string,
    order: "ASC" | "DESC"
): number => {
    const aValue = a[field] != null ? a[field] : ""
    const bValue = b[field] != null ? b[field] : ""
    if (typeof aValue === "string" && typeof bValue === "string") {
        return order === "ASC" ? aValue.localeCompare(bValue) : bValue.localeCompare(aValue)
    }
    if (aValue < bValue) return order === "ASC" ? -1 : 1
    if (aValue > bValue) return order === "ASC" ? 1 : -1
    return 0
}

// A function that sorts an array of records using our custom comparator.
export const customSortData = (
    data: Array<Record<string, any>>,
    sort: {field: string; order: "ASC" | "DESC"}
): Array<Record<string, any>> => {
    return [...data].sort((a, b) => customSortComparator(a, b, sort.field, sort.order))
}
