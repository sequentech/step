// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const getValueFromCookie = (cookieName: string) => {
    const cookies = Object.fromEntries(document.cookie.split("; ").map((c) => c.split("=")))
    const value = cookies[cookieName]

    return value || undefined
}

export function setCookie(
    name: string,
    value: string,
    systemVersion: string = "-",
    domain?: string
) {
    let cookie =
        `${encodeURIComponent(name)}=${encodeURIComponent(value)}` + `; Path=/` + `; SameSite=Lax`

    // Add domain only if defined
    if (domain) {
        cookie += `; Domain=${domain}`
    }

    // Add Secure only if NOT dev
    if (systemVersion !== "-") {
        cookie += `; Secure`
    }
    document.cookie = cookie
}
