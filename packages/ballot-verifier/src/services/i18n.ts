// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {initializeLanguages} from "@sequentech/ui-core"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import spanishInformalTranslation from "../translations/es-tu"
import catalanTranslation from "../translations/cat"
import catalanInformalTranslation from "../translations/cat-tu"
import frenchTranslation from "../translations/fr"
import tagalogTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"

export const getLanguageFromURL = () => {
    const params = new URLSearchParams(window.location.search)
    return params.get("lang") || undefined
}

initializeLanguages(
    {
        "en": englishTranslation,
        "es": spanishTranslation,
        "cat": catalanTranslation,
        "fr": frenchTranslation,
        "tl": tagalogTranslation,
        "gl": galegoTranslation,
        "es-tu": spanishInformalTranslation,
        "cat-tu": catalanInformalTranslation,
    },
    getLanguageFromURL()
)
