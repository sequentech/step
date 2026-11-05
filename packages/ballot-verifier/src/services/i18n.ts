// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ETranslationScope, initializeLanguages} from "@sequentech/ui-core"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import catalanTranslation from "../translations/cat"
import frenchTranslation from "../translations/fr"
import tagalogTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"

export const getLanguageFromURL = () => {
    const params = new URLSearchParams(window.location.search)
    return params.get("lang") || undefined
}

const translations = {
    en: englishTranslation,
    es: spanishTranslation,
    cat: catalanTranslation,
    fr: frenchTranslation,
    tl: tagalogTranslation,
    gl: galegoTranslation,
}

/** Every language the ballot verifier ships. */
export const APP_LANGUAGES = Object.keys(translations)

initializeLanguages(translations, getLanguageFromURL(), ETranslationScope.BALLOT_VERIFIER)
