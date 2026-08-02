// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {getDomain} from "tldts"

export function getValueFromCookie(cookieName: string) {
    const cookies = Object.fromEntries(document.cookie.split("; ").map((c) => c.split("=")))
    const value = cookies[cookieName]

    return value || undefined
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
