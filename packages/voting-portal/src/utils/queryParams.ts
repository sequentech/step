// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Helper function to get cookie value
function getValidLangCookie() {
    const cookies = Object.fromEntries(document.cookie.split("; ").map((c) => c.split("=")))

    const lang = cookies["KEYCLOAK_LANG"]
    if (!lang) return undefined

    return decodeURIComponent(lang)
}

export const getLanguageFromCookie = () => {
    const langFromCookie = getValidLangCookie()

    if (langFromCookie) {
        const newUrl = new URL(window.location.href)
        newUrl.searchParams.set("lang", langFromCookie)
        window.history.replaceState({}, "", newUrl)
        return langFromCookie
    }

    return undefined
}

export const getLanguageFromURL = () => {
    const params = new URLSearchParams(window.location.search)
    return params.get("lang") || undefined
}
