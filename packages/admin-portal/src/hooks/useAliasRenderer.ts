// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {translateFromPresentation} from "@sequentech/ui-core"
import {useCallback} from "react"
import {useTranslation} from "react-i18next"

export const MISSING_ALIAS = "-"

type PlainObject = Record<string, unknown>

const isPlainObject = (value: unknown): value is PlainObject =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const getPresentation = (item: PlainObject): PlainObject | undefined => {
    if (isPlainObject(item.i18n)) return item
    return isPlainObject(item.presentation) ? item.presentation : undefined
}

// The item may have no translation for the UI language or English, so also try
// its own default language and, as a last resort, any language it does have.
const getFallbackLanguages = (item: PlainObject, uiLang: string, defaultLang?: string) => {
    const presentation = getPresentation(item)
    const languageConf = isPlainObject(presentation?.language_conf)
        ? presentation.language_conf
        : undefined
    const itemDefaultLang = languageConf?.default_language_code
    const available = isPlainObject(presentation?.i18n) ? Object.keys(presentation.i18n) : []

    const languages = [
        uiLang,
        defaultLang,
        typeof itemDefaultLang === "string" ? itemDefaultLang : undefined,
        "en",
        ...available,
    ]
    return Array.from(new Set(languages.filter((lang): lang is string => !!lang)))
}

export const translateWithFallback = (
    item: object,
    keys: string[],
    uiLang: string,
    defaultLang?: string
): string | undefined => {
    const record = item as PlainObject
    for (const lang of getFallbackLanguages(record, uiLang, defaultLang)) {
        for (const key of keys) {
            const value = translateFromPresentation(record, key, lang)
            if (value) return value
        }
    }
    return undefined
}

export const renderAlias = (item: unknown, uiLang: string, defaultLang?: string): string => {
    const t = (x: unknown) =>
        (isPlainObject(x) && translateWithFallback(x, ["alias", "name"], uiLang, defaultLang)) ||
        MISSING_ALIAS

    if (item == null) return MISSING_ALIAS

    if (isPlainObject(item)) {
        return t(item)
    }

    const s = String(item).trim()
    if (!s) return MISSING_ALIAS

    if (s.startsWith("{")) {
        try {
            return t(JSON.parse(s))
        } catch {
            return MISSING_ALIAS
        }
    }

    return MISSING_ALIAS
}

export function useAliasRenderer() {
    const {i18n} = useTranslation()

    const aliasRenderer = useCallback(
        (item: unknown, defaultLang?: string) => renderAlias(item, i18n.language, defaultLang),
        [i18n.language]
    )

    return aliasRenderer
}
