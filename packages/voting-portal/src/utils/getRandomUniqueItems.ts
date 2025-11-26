// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const getRandomUniqueItems = (array: any[], n: number) => {
    if (n > array.length) {
        throw new Error("n cannot be greater than the length of the array")
    }

    const result = new Set()
    while (result.size < n) {
        const randomIndex = Math.floor(Math.random() * array.length)
        result.add(array[randomIndex])
    }

    return Array.from(result)
}
