// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const formatNumber = (value: number | string): string => {
    // Ensure the input is a number, or parse it from a string
    let numberValue = 0
    if (typeof value === "string") {
        try {
            numberValue = parseFloat(value)
        } catch (e) {
            return value
        }
    } else {
        numberValue = value
    }

    // Return formatted number or handle invalid inputs gracefully
    // This can happen when the string is for example `-`
    if (isNaN(numberValue)) {
        return value.toString() // Fallback: return the original value
    }

    return new Intl.NumberFormat("en-US").format(numberValue)
}
