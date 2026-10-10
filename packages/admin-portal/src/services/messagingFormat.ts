// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback} from "react"
import {useTranslation} from "react-i18next"

/**
 * The Intl locale of a portal language: the portals name Catalan "cat" and
 * Tagalog "tl", which Intl knows as "ca" and "fil".
 */
export const intlLocale = (language?: string | null): string => {
    const base = (language || "en").split("-")[0]
    if (base === "cat") return "ca"
    if (base === "tl") return "fil"
    return language || "en"
}

/** A count written the way the viewer's language writes numbers. */
export const formatCount = (value: number, language?: string | null): string => {
    try {
        return new Intl.NumberFormat(intlLocale(language)).format(value)
    } catch {
        return String(value)
    }
}

/** [`formatCount`] in the portal's current language. */
export const useFormatCount = (): ((value: number) => string) => {
    const {i18n} = useTranslation()
    const language = i18n?.language
    return useCallback((value: number) => formatCount(value, language), [language])
}
