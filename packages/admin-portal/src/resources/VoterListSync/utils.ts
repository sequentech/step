// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Formats a `GeneratedAt` unix-seconds value for display, with `format`
 * (one value with its zone label, `useZonedFormat`).
 */
export const formatGeneratedAt = (
    unixSeconds: number,
    format: (value: number) => string
): string => {
    if (!unixSeconds) {
        return "-"
    }
    return format(unixSeconds * 1000)
}
