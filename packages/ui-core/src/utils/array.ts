// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {shuffle as DashShuffle} from "moderndash"

export const shuffle = DashShuffle

export const splitList = <T>(
    list: Array<T>,
    test: (element: T) => boolean
): [Array<T>, Array<T>] => {
    const negative: Array<T> = []
    const positive: Array<T> = []

    for (let element of list) {
        if (test(element)) {
            positive.push(element)
        } else {
            negative.push(element)
        }
    }

    return [negative, positive]
}

export const keyBy = <T>(list: Array<T>, id: string): Record<string, T> => {
    let record: Record<string, T> = {}
    for (let element of list) {
        record[element[id]] = element
    }
    return record
}

/**
 * Lodash-like keyByReturn implementation - creates an object keyed by the specified property
 * or function result for each item in the array.
 *
 * @param {Array} array - The array to convert to an object
 * @param {String|Function} iteratee - Property name or function to generate keys
 * @returns {Object} - Object with values keyed by iteratee result
 */
export const keyByReturn = <T>(
    array: Array<T>,
    iteratee: keyof T | ((item: T) => string)
): Record<string, T> => {
    // Handle empty arrays
    if (!array || !array.length) {
        return {}
    }

    const result: Record<string, T> = {}
    const isFunction = typeof iteratee === "function"

    // Process each item in the array
    for (let i = 0; i < array.length; i++) {
        const item = array[i]
        // Get the key by calling the function or accessing the property
        const key = isFunction
            ? (iteratee as (item: T) => string)(item)
            : String(item[iteratee as keyof T])

        // Only add defined keys to the result
        if (key !== undefined && key !== null) {
            result[key] = item
        }
    }

    return result
}
