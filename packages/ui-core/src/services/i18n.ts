// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import i18n, {i18n as I18N, InitOptions, Resource} from "i18next"
import {deepmerge} from "@mui/utils"
import LanguageDetector from "i18next-browser-languagedetector"
import {initReactI18next} from "react-i18next"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import catalanTranslation from "../translations/cat"
import frenchTranslation from "../translations/fr"
import tagalogTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"
import dutchTranslation from "../translations/nl"
import basqueTranslation from "../translations/eu"
import {IElectionEventPresentation} from "../types/ElectionEventPresentation"

export const initializeLanguages = (externalTranslations: Resource, language?: string) => {
    const libTranslations: Resource = {
        en: englishTranslation,
        es: spanishTranslation,
        cat: catalanTranslation,
        fr: frenchTranslation,
        tl: tagalogTranslation,
        gl: galegoTranslation,
        nl: dutchTranslation,
        eu: basqueTranslation,
    }
    const mergedTranslations = deepmerge(libTranslations, externalTranslations)
    const i18nConfig: InitOptions = {
        // we init with resources
        resources: mergedTranslations,
        fallbackLng: "en",
        lng: language || undefined, // Use provided language or fallback to english if not available
        debug: true,

        // have a common namespace used around the full app
        ns: ["translations"],
        defaultNS: "translations",

        keySeparator: ".",

        interpolation: {
            escapeValue: false,
        },
        react: {
            transKeepBasicHtmlNodesFor: ["ol", "li", "p", "br", "strong"],
        },
    }
    if (language) {
        i18n.use(initReactI18next).init(i18nConfig) // If a language is explicitly provided, don't use LanguageDetector
    } else {
        i18n.use(LanguageDetector).use(initReactI18next).init(i18nConfig) // Use LanguageDetector if no language is explicitly provided
    }
}

export const getLanguages = (i18n: I18N) => Object.keys(i18n.services.resourceStore.data)

export const overwriteTranslations = (
    electionEventPresentation: IElectionEventPresentation | undefined,
    changeDefaultLanguage: boolean = true
): boolean => {
    // Check object has translations to overwrite
    let hasChangedDefaultLanguage = false
    const i18nObj = electionEventPresentation?.i18n
    if (!i18nObj) {
        return hasChangedDefaultLanguage
    }

    Object.keys(i18nObj).forEach((lang) => {
        const translations = i18nObj[lang]
        const currentResources = i18n.getResourceBundle(lang, "translations") || {}

        // Convert dot notation to nested objects
        const nestedTranslations = {}
        Object.keys(translations).forEach((key) => {
            const keys = key.split(".")
            keys.reduce((acc, k, i) => {
                return (acc[k] = i === keys.length - 1 ? translations[key] : acc[k] || {})
            }, nestedTranslations)
        })

        const mergedResources = deepmerge(currentResources, nestedTranslations)

        i18n.addResourceBundle(lang, "translations", mergedResources, true, true) // Overwriting existing resource for language
    })

    if (changeDefaultLanguage) {
        let languageConf = electionEventPresentation?.language_conf
        let enabledLanguages = languageConf?.enabled_language_codes ?? ["en"]
        let defaultLanguage = languageConf?.default_language_code
        let currentLanguage = i18n.language
        if (
            !!enabledLanguages &&
            !!defaultLanguage &&
            defaultLanguage !== currentLanguage &&
            enabledLanguages.includes(defaultLanguage)
        ) {
            i18n.changeLanguage(defaultLanguage)
            hasChangedDefaultLanguage = true
        }
    }
    return hasChangedDefaultLanguage
}

export default i18n
