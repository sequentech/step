// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** Formats a `GeneratedAt` unix-seconds value for display. */
export const formatGeneratedAt = (unixSeconds: number): string => {
    if (!unixSeconds) {
        return "-"
    }
    return new Date(unixSeconds * 1000).toLocaleString()
}
