// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {initializeLanguages} from "@sequentech/ui-core"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import catalanTranslation from "../translations/cat"
import frenchTranslation from "../translations/fr"
import tagalotTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"
import dutchTranslation from "../translations/nl"
import basqueTranslation from "../translations/eu"

/**
 * Lazy i18n bootstrap. Do NOT initialize i18n on module import to avoid
 * first-paint with an incorrect language. Consumers must call initI18n()
 * with a resolved initial language prior to rendering UI.
 */
let initialized = false
let initPromise: Promise<void> | null = null

export const initI18n = (language?: string): Promise<void> => {
    if (initialized) return Promise.resolve()
    if (initPromise) return initPromise

    initPromise = new Promise<void>((resolve) => {
        initializeLanguages(
            {
                en: englishTranslation,
                es: spanishTranslation,
                cat: catalanTranslation,
                fr: frenchTranslation,
                tl: tagalotTranslation,
                gl: galegoTranslation,
                nl: dutchTranslation,
                eu: basqueTranslation,
            },
            language
        )
        initialized = true
        resolve()
    })

    return initPromise
}

export const isI18nInitialized = () => initialized
