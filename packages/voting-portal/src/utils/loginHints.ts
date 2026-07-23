// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const LOGIN_HINT_PREFIX = "login_hint__"
export const MAX_LOGIN_HINT_COUNT = 5
export const MAX_LOGIN_HINT_NAME_LENGTH = 128
export const MAX_LOGIN_HINT_VALUE_LENGTH = 255

const LOGIN_HINT_NAME_PATTERN = /^[A-Za-z0-9._-]+$/

export type LoginHints = Record<string, string>

export type ParsedLoginHints = {
    hints: LoginHints
    remainingSearch: string
}

export class InvalidLoginHintsError extends Error {
    constructor() {
        super("Invalid login hint parameters")
        this.name = "InvalidLoginHintsError"
    }
}

const validateLoginHint = (name: string, value: string): void => {
    if (
        !name ||
        name.length > MAX_LOGIN_HINT_NAME_LENGTH ||
        !LOGIN_HINT_NAME_PATTERN.test(name) ||
        !value.trim() ||
        value.length > MAX_LOGIN_HINT_VALUE_LENGTH
    ) {
        throw new InvalidLoginHintsError()
    }
}

export const parseLoginHints = (search: string): ParsedLoginHints => {
    const searchParams = new URLSearchParams(search)
    const hints: LoginHints = {}

    for (const [parameterName, value] of searchParams.entries()) {
        if (!parameterName.startsWith(LOGIN_HINT_PREFIX)) {
            continue
        }

        const hintName = parameterName.slice(LOGIN_HINT_PREFIX.length)
        validateLoginHint(hintName, value)

        if (Object.hasOwn(hints, hintName)) {
            throw new InvalidLoginHintsError()
        }

        hints[hintName] = value
        if (Object.keys(hints).length > MAX_LOGIN_HINT_COUNT) {
            throw new InvalidLoginHintsError()
        }
    }

    for (const hintName of Object.keys(hints)) {
        searchParams.delete(`${LOGIN_HINT_PREFIX}${hintName}`)
    }

    const remainingSearch = searchParams.toString()
    return {
        hints,
        remainingSearch: remainingSearch ? `?${remainingSearch}` : "",
    }
}

export const appendLoginHints = (url: string, hints: LoginHints): string => {
    const result = new URL(url)
    const hintEntries = Object.entries(hints)

    if (hintEntries.length > MAX_LOGIN_HINT_COUNT) {
        throw new InvalidLoginHintsError()
    }

    for (const [name, value] of hintEntries) {
        validateLoginHint(name, value)
        result.searchParams.set(`${LOGIN_HINT_PREFIX}${name}`, value)
    }

    return result.toString()
}
