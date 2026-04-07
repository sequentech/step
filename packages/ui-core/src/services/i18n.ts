// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
import {ELanguageDetectionPolicy, ILanguageConf} from "@root/types/LanguageConf"
import {getValueFromCookie} from "@root/utils/cookies"

export const KEYCLOAK_LANG_COOKIE_NAME = "KEYCLOAK_LANG"

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

    // BCP 47-compliant tags initially and on language changes
    const toBCP47 = (lang: string): string => {
        // Map internal/non-standard codes to valid BCP 47 when needed
        const map: Record<string, string> = {
            cat: "ca", // Catalan
        }
        const candidate = map[lang] || lang

        // Simple BCP 47 normalization: lowercase language, uppercase country
        // e.g., "en-us" -> "en-US", "ES-es" -> "es-ES"
        const parts = candidate.split("-")
        if (parts.length === 1) {
            return parts[0].toLowerCase()
        }
        const [language, ...rest] = parts
        const normalizedRest = rest.map((part, index) => {
            // Country codes (2 letters) should be uppercase, others lowercase
            return part.length === 2 && index === 0 ? part.toUpperCase() : part.toLowerCase()
        })
        return [language.toLowerCase(), ...normalizedRest].join("-")
    }

    const updateHtmlLang = (lng?: string) => {
        if (typeof document === "undefined") return
        const tag = toBCP47(lng || i18n.language || "en")
        document.documentElement.setAttribute("lang", tag)
    }

    // Initial set and subscribe to changes
    updateHtmlLang(language)
    i18n.on("languageChanged", updateHtmlLang)
}

export const getLanguages = (i18n: I18N) => Object.keys(i18n.services.resourceStore.data)

/// Applies language detection policy defined in language config, if any.
export const applyLanguagePolicy = (languageConf: ILanguageConf | undefined): boolean => {
    if (!languageConf || !languageConf.language_detection_policy) {
        return false
    }

    const {language_detection_policy, default_language_code} = languageConf

    // If policy exists and equals FORCE_DEFAULT, force default language
    if (
        language_detection_policy === ELanguageDetectionPolicy.FORCE_DEFAULT &&
        default_language_code
    ) {
        i18n.changeLanguage(default_language_code)
        return true
    }

    return false
}

/// Applies language policy defined in election event presentation, if any
/// Url search param "lang" > cookie > presentation policy > browser settings
/// The Url search param "lang" is checked in i18n initialization.
export const applyPresentationLanguagePolicy = (
    presentation: IElectionEventPresentation | undefined
): boolean => {
    if (!presentation?.language_conf) {
        return false
    }

    // If query param "lang" exists, skip applying presentation policy to allow manual override
    if (typeof window !== "undefined") {
        const params = new URLSearchParams(window.location.search)
        if (params.get("lang")) {
            return false
        }
    }

    let langfromCookie: string | undefined = getValueFromCookie(KEYCLOAK_LANG_COOKIE_NAME)
    if (langfromCookie) {
        i18n.changeLanguage(langfromCookie)
        return true
    }
    return applyLanguagePolicy(presentation.language_conf)
}

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
        // Apply language policy: skip if query param provided, otherwise check for FORCE_DEFAULT
        hasChangedDefaultLanguage = applyPresentationLanguagePolicy(electionEventPresentation)
    }
    return hasChangedDefaultLanguage
}

export default i18n
