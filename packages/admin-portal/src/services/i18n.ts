// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {initializeLanguages, isString, overwriteTranslations} from "@sequentech/ui-core"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import catalanTranslation from "../translations/cat"
import frenchTranslation from "../translations/fr"
import tagalogTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"
import dutchTranslation from "../translations/nl"
import i18next from "i18next"

type I18N = Record<string, Record<string, string>>

initializeLanguages({
    en: englishTranslation,
    es: spanishTranslation,
    cat: catalanTranslation,
    fr: frenchTranslation,
    tl: tagalogTranslation,
    gl: galegoTranslation,
    nl: dutchTranslation,
})

export const triggerOverrideTranslations = (i18n: I18N) => {
    initializeLanguages({
        en: englishTranslation,
        es: spanishTranslation,
        cat: catalanTranslation,
        fr: frenchTranslation,
        tl: tagalogTranslation,
        gl: galegoTranslation,
        nl: dutchTranslation,
    })
    overwriteTranslations({presentation: {i18n}})
}

export const getAllLangs = (): Array<string> => ["en", "es", "cat", "fr", "tl", "gl", "nl"]

export const addDefaultTranslationsToElement = (data: {
    name?: string | null
    description?: string | null
    alias?: string | null
}): Record<string, Record<string, string>> => {
    let i18n: Record<string, Record<string, string>> = {}
    let langs = getAllLangs()

    for (let lang of langs) {
        let i18nLang: Record<string, string> = {}
        if (isString(data.name)) {
            i18nLang["name"] = data.name
        }
        if (isString(data.description)) {
            i18nLang["description"] = data.description
        }
        if (isString(data.alias)) {
            i18nLang["alias"] = data.alias
        }
        i18n[lang] = i18nLang
    }
    return i18n
}

export default i18next
