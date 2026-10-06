// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETranslationScope, initializeLanguages} from "@sequentech/ui-core"
import englishTranslation from "@/translations/en"
import spanishTranslation from "@/translations/es"
import catalanTranslation from "@/translations/cat"
import frenchTranslation from "@/translations/fr"
import tagalogTranslation from "@/translations/tl"
import galegoTranslation from "@/translations/gl"
import dutchTranslation from "@/translations/nl"
import basqueTranslation from "@/translations/eu"

export const RESULTS_PORTAL_LANGUAGES = ["en", "es", "cat", "fr", "tl", "gl", "nl", "eu"]

const getLanguageFromURL = () => {
    if (typeof window === "undefined") {
        return undefined
    }

    const params = new URLSearchParams(window.location.search)
    return params.get("lang") || undefined
}

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
    getLanguageFromURL(),
    ETranslationScope.RESULTS_PORTAL
)
