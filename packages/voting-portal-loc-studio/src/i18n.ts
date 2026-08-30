// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {initializeLanguages, i18n} from "@sequentech/ui-core"
import {deepmerge} from "@mui/utils"
import englishTranslation from "@voting-portal/translations/en"
import spanishTranslation from "@voting-portal/translations/es"
import catalanTranslation from "@voting-portal/translations/cat"
import frenchTranslation from "@voting-portal/translations/fr"
import tagalogTranslation from "@voting-portal/translations/tl"
import galegoTranslation from "@voting-portal/translations/gl"
import dutchTranslation from "@voting-portal/translations/nl"
import basqueTranslation from "@voting-portal/translations/eu"
import {flattenTranslations, nestTranslation, NestedTranslations} from "./translations"
import {wrapTranslation} from "./markers"

export const LOC_STUDIO_LANGUAGES = ["en", "es", "cat", "fr", "tl", "gl", "nl", "eu"] as const

export type LocStudioLanguage = (typeof LOC_STUDIO_LANGUAGES)[number]

export type OverridesByLanguage = Record<string, Record<string, string>>

const usedKeys = new Set<string>()
const originalBundles: Record<string, Record<string, string>> = {}

export const beginKeyCapture = (): void => {
    usedKeys.clear()
}

export const getOriginalValue = (language: string, key: string): string | undefined =>
    originalBundles[language]?.[key]

export const getOriginalBundle = (language: string): Record<string, string> =>
    originalBundles[language] || {}

export const getCapturedKeys = (): string[] => Array.from(usedKeys).sort()

const instrumentTranslator = (): void => {
    const translator = (
        i18n as unknown as {
            translator?: {
                translate: (keys: string | string[], options?: unknown) => string
            }
        }
    ).translator
    if (!translator || typeof translator.translate !== "function") {
        return
    }
    const original = translator.translate.bind(translator)
    translator.translate = (keys: string | string[], options?: unknown) => {
        const key = Array.isArray(keys) ? keys[0] : keys
        const value = original(keys, options)
        if (typeof key === "string" && key.length > 0) {
            usedKeys.add(key)
        }
        return typeof key === "string" && typeof value === "string"
            ? wrapTranslation(key, value)
            : value
    }
}

export const initializeLocStudioI18n = (): void => {
    initializeLanguages(
        {
            en: englishTranslation,
            es: spanishTranslation,
            cat: catalanTranslation,
            fr: frenchTranslation,
            tl: tagalogTranslation,
            gl: galegoTranslation,
            nl: dutchTranslation,
            eu: basqueTranslation,
        },
        "en"
    )
    const capture = () => {
        instrumentTranslator()
        LOC_STUDIO_LANGUAGES.forEach((language) => {
            originalBundles[language] = getBundleForLanguage(language)
        })
    }
    if (i18n.isInitialized) {
        capture()
    } else {
        i18n.on("initialized", capture)
    }
}

export const getBundleForLanguage = (language: string): Record<string, string> => {
    const bundle = (i18n.getResourceBundle(language, "translations") || {}) as NestedTranslations
    return flattenTranslations(bundle)
}

export const applyOverride = (language: string, key: string, value: string): void => {
    const nested = nestTranslation(key, value)
    const current = (i18n.getResourceBundle(language, "translations") || {}) as NestedTranslations
    i18n.addResourceBundle(language, "translations", deepmerge(current, nested), true, true)
}

export const applyOverrides = (language: string, overrides: Record<string, string>): void => {
    Object.entries(overrides).forEach(([key, value]) => {
        applyOverride(language, key, value)
    })
}
