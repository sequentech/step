// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const MAX_CLASSNAME_LENGTH = 40
export const ROOT_CLASS_PREFIX = "e-"

/**
 * Transforms a string to a valid class name.
 * 1. No spaces.
 * 2. Only a-zA-Z0-9 and -_
 * 3. Must start with a letter.
 * 4. Max length based on MAX_CLASSNAME_LENGTH
 */
export function toValidClassName(input: string, prefix = ROOT_CLASS_PREFIX): string {
    let sanitized = input.replace(/\s+/g, "").replace(/[^a-zA-Z0-9-_]/g, "")

    let result = `${prefix}${sanitized}`
    result = result.slice(0, MAX_CLASSNAME_LENGTH)

    return result
}

export function getContestClassName(externalId?: string | null): string {
    const prefix = "c-"
    const className = toValidClassName(externalId ?? "", prefix)
    return className === prefix ? "" : className
}
