// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import i18n, {i18n as I18N, InitOptions, Resource} from "i18next"
import {deepmerge} from "@mui/utils"
import LanguageDetector from "i18next-browser-languagedetector"
import {initReactI18next} from "react-i18next"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import spanishInformalTranslation from "../translations/es-tu"
import catalanTranslation from "../translations/cat"
import catalanInformalTranslation from "../translations/cat-tu"
import frenchTranslation from "../translations/fr"
import tagalogTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"
import dutchTranslation from "../translations/nl"
import basqueTranslation from "../translations/eu"
import {IElectionEventPresentation} from "../types/ElectionEventPresentation"
import {ELanguageDetectionPolicy, ILanguageConf} from "@root/types/LanguageConf"
import {getValueFromCookie} from "@root/utils/cookies"
import {iso_639_2t_to_bcp47_js, locale_to_internal_language_code_js} from "sequent-core"
import {ITenantSettings} from "@root/types/TenantSettings"
import {INFORMAL_SUFFIX, splitRegister, withRegister, withRegisterBCP47} from "./registerLocale"

export const USER_LANGUAGE_COOKIE_NAME = "USER_LANGUAGE"

/**
 * Minimal fallback used during app bootstrap before the WASM module is ready.
 * The only current frontend/internal mismatch is Catalan (`ca` vs `cat`).
 */
const normalizeLanguageCodeFallback = (lang: string): string => {
    const {base, informal} = splitRegister(lang.toLowerCase())
    return withRegister(base === "ca" ? "cat" : base, informal)
}

/**
 * Normalizes external locale inputs (query params, cookies, Keycloak locale
 * values) into the internal language codes used by the frontends.
 *
 * Accepts the internal code (`es-tu`) and the private-use tag the DOM carries
 * (`es-x-tu`), as well as plain BCP 47 and ISO 639-2/T input. The register
 * marker is split off here rather than inside sequent-core because the WASM
 * bindings ship as a prebuilt tarball; `locale.rs` knows the same rule, so the
 * two agree once that artifact is next rebuilt.
 */
export const normalizeLanguageCode = (lang?: string): string | undefined => {
    if (!lang) {
        return undefined
    }

    const {base, informal} = splitRegister(lang)
    let normalized: string
    try {
        normalized = locale_to_internal_language_code_js(base)
    } catch {
        return normalizeLanguageCodeFallback(lang)
    }
    return withRegister(normalized, informal)
}

/**
 * Converts an internal language code to a BCP 47-compliant tag via the WASM
 * function defined in sequent-core. If WASM is not yet initialised (e.g. during
 * module evaluation at startup), returns the input unchanged and the BCP 47
 * normalisation will be applied again on the next `languageChanged` event.
 */
export const toBCP47 = (lang: string): string => {
    const {base, informal} = splitRegister(lang)
    let candidate: string
    try {
        candidate = iso_639_2t_to_bcp47_js(base)
    } catch {
        // WASM not yet initialised; return the input unchanged.
        return lang
    }
    if (informal) {
        return withRegisterBCP47(candidate, true)
    }
    const parts = candidate.split("-")
    if (parts.length === 1) {
        return parts[0].toLowerCase()
    }
    const [language, ...rest] = parts
    const normalizedRest = rest.map((part, index) =>
        part.length === 2 && index === 0 ? part.toUpperCase() : part.toLowerCase()
    )
    return [language.toLowerCase(), ...normalizedRest].join("-")
}

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
        [`es-${INFORMAL_SUFFIX}`]: spanishInformalTranslation,
        [`cat-${INFORMAL_SUFFIX}`]: catalanInformalTranslation,
    }
    const mergedTranslations = deepmerge(libTranslations, externalTranslations)
    const resolvedLanguage = normalizeLanguageCode(language)
    const i18nConfig: InitOptions = {
        // we init with resources
        resources: mergedTranslations,
        // The register variants are complete bundles, so this only matters for
        // a key added to a base file and not yet to its variant: falling back
        // to the base language beats dropping the voter into English.
        fallbackLng: {
            [`es-${INFORMAL_SUFFIX}`]: ["es", "en"],
            [`cat-${INFORMAL_SUFFIX}`]: ["cat", "en"],
            default: ["en"],
        },
        lng: resolvedLanguage || undefined, // Use provided language or fallback to english if not available
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
    if (resolvedLanguage) {
        i18n.use(initReactI18next).init(i18nConfig) // If a language is explicitly provided, don't use LanguageDetector
    } else {
        i18n.use(LanguageDetector).use(initReactI18next).init(i18nConfig) // Use LanguageDetector if no language is explicitly provided
    }

    const updateHtmlLang = (lng?: string) => {
        if (typeof document === "undefined") return
        const tag = toBCP47(lng || i18n.language || "en")
        document.documentElement.setAttribute("lang", tag)
    }

    // Initial set and subscribe to changes
    updateHtmlLang(resolvedLanguage)
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
        i18n.changeLanguage(normalizeLanguageCode(default_language_code) || default_language_code)
        return true
    }

    return false
}

/// Applies language policy defined in election event presentation or tenant settings, if any
/// Url search param "lang" > user selected locale (saved in cookie) >  language detection policy > browser settings
/// The Url search param "lang" is checked in i18n initialization.
export const applyConfigurationLanguagePolicy = (
    config: IElectionEventPresentation | ITenantSettings | undefined
): boolean => {
    if (!config?.language_conf) {
        return false
    }

    // If query param "lang" exists, skip applying presentation policy to allow manual override
    if (typeof window !== "undefined") {
        const params = new URLSearchParams(window.location.search)
        if (params.get("lang")) {
            return false
        }
    }
    let cookieLang: string | undefined
    cookieLang = getValueFromCookie(USER_LANGUAGE_COOKIE_NAME)

    if (cookieLang) {
        i18n.changeLanguage(normalizeLanguageCode(cookieLang) || cookieLang)
        return true
    }

    return applyLanguagePolicy(config.language_conf)
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
        hasChangedDefaultLanguage = applyConfigurationLanguagePolicy(electionEventPresentation)
    }
    return hasChangedDefaultLanguage
}

export default i18n
