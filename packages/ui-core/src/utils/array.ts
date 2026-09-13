// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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

export const keyBy = <T>(list: Array<T>, id: keyof T): Record<string, T> => {
    // Identifiers come from election data, so names such as __proto__ must
    // behave like ordinary keys rather than invoking Object's inherited setter.
    const record: Record<string, T> = Object.create(null)
    for (const element of list) {
        record[String(element[id])] = element
    }
    return record
}
