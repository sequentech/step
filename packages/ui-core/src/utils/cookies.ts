// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {getDomain} from "tldts"

/** Decode our own cookies while tolerating legacy values with literal percent signs. */
function decodeCookiePart(value: string): string {
    try {
        return decodeURIComponent(value)
    } catch {
        return value
    }
}

export function getValueFromCookie(cookieName: string): string | undefined {
    for (const entry of document.cookie.split(";")) {
        const separator = entry.indexOf("=")
        if (separator < 0) continue
        const name = decodeCookiePart(entry.slice(0, separator).trim())
        if (name === cookieName) {
            // Only the first equals sign separates name from value. Opaque
            // tokens may contain further equals signs as base64 padding.
            return decodeCookiePart(entry.slice(separator + 1)) || undefined
        }
    }
    return undefined
}

export function setCookie(name: string, value: string) {
    // Extract the parent domain from the current hostname.
    const hostname = window.location.hostname
    const domain = getDomain(hostname) || ""

    let cookie =
        `${encodeURIComponent(name)}=${encodeURIComponent(value)}` + `; Path=/` + `; SameSite=Lax`

    if (domain) {
        cookie += `; Domain=${domain}`
    }

    if (window.location.protocol === "https:") {
        cookie += `; Secure`
    }

    document.cookie = cookie
}
