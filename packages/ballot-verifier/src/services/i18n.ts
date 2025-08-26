// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {initializeLanguages} from "@sequentech/ui-core"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import catalanTranslation from "../translations/cat"
import frenchTranslation from "../translations/fr"
import tagalogTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"

// Lazy initialization (same pattern as voting-portal)
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
                tl: tagalogTranslation,
                gl: galegoTranslation,
            },
            language
        )
        initialized = true
        resolve()
    })

    return initPromise
}

export const isI18nInitialized = () => initialized
