// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const cssClassToken = (value: string | number | null | undefined): string => {
    if (value === null || value === undefined || String(value).length === 0) {
        return "global"
    }

    return String(value).replace(/[^a-zA-Z0-9_-]/g, "-")
}

export const entityClassName = (
    entity: string,
    value: string | number | null | undefined
): string => `seq-results-${entity}--${cssClassToken(value)}`
