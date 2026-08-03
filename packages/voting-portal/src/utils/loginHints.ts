// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const LOGIN_HINT_PREFIX = "login_hint__"
export const MAX_LOGIN_HINT_COUNT = 5
export const MAX_LOGIN_HINT_NAME_LENGTH = 128
export const MAX_LOGIN_HINT_VALUE_LENGTH = 255

const LOGIN_HINT_NAME_PATTERN = /^[A-Za-z0-9._-]+$/
const HEX_PAIR_PATTERN = /^[0-9A-Fa-f]{2}$/

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

const ENROLL_ROUTE_PATTERN = /\/enroll\/?$/

/**
 * Returns where Keycloak sends the voter once enrollment succeeds, or undefined to keep the
 * adapter default. Kept beside {@link routeAcceptsLoginHints} because both must accept the same
 * optional trailing slash: a link good enough to carry hints has to reach /login afterwards.
 */
export const enrollmentRedirectUrl = (baseUrl: string, search: string): string | undefined =>
    ENROLL_ROUTE_PATTERN.test(baseUrl)
        ? baseUrl.replace(ENROLL_ROUTE_PATTERN, "/login") + search
        : undefined

// Decode only enough ASCII bytes to recognize the namespace. This remains reliable when a later
// percent escape is malformed, which is precisely when the invalid-link cleanup must fail closed.
const rawNameHasLoginHintPrefix = (rawName: string): boolean => {
    let rawIndex = 0

    for (let prefixIndex = 0; prefixIndex < LOGIN_HINT_PREFIX.length; prefixIndex += 1) {
        if (rawIndex >= rawName.length) {
            return false
        }

        let character = rawName[rawIndex]
        if (character === "%") {
            const hexPair = rawName.slice(rawIndex + 1, rawIndex + 3)
            if (!HEX_PAIR_PATTERN.test(hexPair)) {
                return false
            }
            character = String.fromCharCode(Number.parseInt(hexPair, 16))
            rawIndex += 3
        } else {
            character = character === "+" ? " " : character
            rawIndex += 1
        }

        if (character !== LOGIN_HINT_PREFIX[prefixIndex]) {
            return false
        }
    }

    return true
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

const validateLoginHintEncoding = (search: string): void => {
    for (const component of search.replace(/^\?/, "").split("&")) {
        const separatorIndex = component.indexOf("=")
        const rawName = separatorIndex === -1 ? component : component.slice(0, separatorIndex)
        const rawValue = separatorIndex === -1 ? "" : component.slice(separatorIndex + 1)

        let parameterName: string
        try {
            parameterName = decodeURIComponent(rawName.replaceAll("+", " "))
        } catch {
            if (rawNameHasLoginHintPrefix(rawName)) {
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

    // The portal is compiled targeting ES5, where `for ... of` over an iterator
    // (as opposed to an array) is transpiled into a no-op indexed loop.
    searchParams.forEach((value, parameterName) => {
        if (!parameterName.startsWith(LOGIN_HINT_PREFIX)) {
            return
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
    })

    hints.forEach((_value, hintName) => {
        searchParams.delete(`${LOGIN_HINT_PREFIX}${hintName}`)
    })

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

            return !rawNameHasLoginHintPrefix(rawName)
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
