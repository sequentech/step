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

export const routeAcceptsLoginHints = (pathname: string): boolean =>
    /\/(login|enroll)\/?$/.test(pathname)

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

const validateLoginHintEncoding = (search: string): void => {
    for (const component of search.replace(/^\?/, "").split("&")) {
        const separatorIndex = component.indexOf("=")
        const rawName = separatorIndex === -1 ? component : component.slice(0, separatorIndex)
        const rawValue = separatorIndex === -1 ? "" : component.slice(separatorIndex + 1)

        let parameterName: string
        try {
            parameterName = decodeURIComponent(rawName.replaceAll("+", " "))
        } catch {
            if (rawName.startsWith(LOGIN_HINT_PREFIX)) {
                throw new InvalidLoginHintsError()
            }
            continue
        }

        if (parameterName.startsWith(LOGIN_HINT_PREFIX)) {
            try {
                decodeURIComponent(rawValue.replaceAll("+", " "))
            } catch {
                throw new InvalidLoginHintsError()
            }
        }
    }
}

export const parseLoginHints = (search: string): ParsedLoginHints => {
    validateLoginHintEncoding(search)
    const searchParams = new URLSearchParams(search)
    const hints = new Map<string, string>()

    for (const [parameterName, value] of searchParams.entries()) {
        if (!parameterName.startsWith(LOGIN_HINT_PREFIX)) {
            continue
        }

        const hintName = parameterName.slice(LOGIN_HINT_PREFIX.length)
        validateLoginHint(hintName, value)

        if (hints.has(hintName)) {
            throw new InvalidLoginHintsError()
        }

        hints.set(hintName, value)
        if (hints.size > MAX_LOGIN_HINT_COUNT) {
            throw new InvalidLoginHintsError()
        }
    }

    for (const hintName of hints.keys()) {
        searchParams.delete(`${LOGIN_HINT_PREFIX}${hintName}`)
    }

    const remainingSearch = searchParams.toString()
    return {
        hints: Object.fromEntries(hints),
        remainingSearch: remainingSearch ? `?${remainingSearch}` : "",
    }
}

export const removeLoginHintsFromSearch = (search: string): string => {
    const remainingComponents = search
        .replace(/^\?/, "")
        .split("&")
        .filter((component) => {
            const separatorIndex = component.indexOf("=")
            const rawName = separatorIndex === -1 ? component : component.slice(0, separatorIndex)

            if (rawName.startsWith(LOGIN_HINT_PREFIX)) {
                return false
            }

            try {
                return !decodeURIComponent(rawName.replaceAll("+", " ")).startsWith(
                    LOGIN_HINT_PREFIX
                )
            } catch {
                return true
            }
        })

    return remainingComponents.length > 0 ? `?${remainingComponents.join("&")}` : ""
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
