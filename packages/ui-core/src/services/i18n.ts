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
import {ELanguageDetectionPolicy, ILanguageConf} from "../types/LanguageConf"
import {getValueFromCookie} from "../utils/cookies"
import {iso_639_2t_to_bcp47_js, locale_to_internal_language_code_js} from "sequent-core"
import {ETranslationScope, filterTranslationOverrides} from "./translationScopes"

export const USER_LANGUAGE_COOKIE_NAME = "USER_LANGUAGE"

interface IAppliedTranslationOverride {
    key: string
    language: string
    previousValue: unknown
}

const appliedTranslationOverrides = new Map<ETranslationScope, IAppliedTranslationOverride[]>()

const cloneTranslationResource = (value: unknown): unknown =>
    typeof value === "object" && value !== null ? deepmerge({}, value) : value

interface ITranslationConfiguration {
    i18n?: Record<string, Record<string, string>>
    language_conf?: ILanguageConf
}

/**
 * Minimal fallback used during app bootstrap before the WASM module is ready.
 * The only current frontend/internal mismatch is Catalan (`ca` vs `cat`).
 */
const normalizeLanguageCodeFallback = (lang: string): string => {
    const primary = lang.toLowerCase().split("-")[0]
    return primary === "ca" ? "cat" : primary
}

/**
 * Normalizes external locale inputs (query params, cookies, Keycloak locale
 * values) into the internal language codes used by the frontends.
 */
export const normalizeLanguageCode = (lang?: string): string | undefined => {
    if (!lang) {
        return undefined
    }

    try {
        return locale_to_internal_language_code_js(lang)
    } catch {
        return normalizeLanguageCodeFallback(lang)
    }
}

/**
 * Converts an ISO 639-2/T code to a BCP 47-compliant tag via the WASM function
 * defined in sequent-core. If WASM is not yet initialised (e.g. during module
 * evaluation at startup), returns the input unchanged and the BCP 47 normalisation
 * will be applied again on the next `languageChanged` event.
 */
export const toBCP47 = (lang: string): string => {
    let candidate: string
    try {
        candidate = iso_639_2t_to_bcp47_js(lang)
    } catch {
        // WASM not yet initialised; return the input unchanged.
        return lang
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
    // Reinitialization replaces the complete resource store, so there is no
    // earlier scoped layer left to restore.
    appliedTranslationOverrides.clear()
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
    const resolvedLanguage = normalizeLanguageCode(language)
    const i18nConfig: InitOptions = {
        // we init with resources
        resources: mergedTranslations,
        fallbackLng: "en",
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
            // Scoped overrides are installed after portal data loads. Subscribe React
            // consumers to resource-store additions so already-mounted screens rerender.
            bindI18nStore: "added",
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
    config: ITranslationConfiguration | undefined
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
    config: ITranslationConfiguration | undefined,
    {
        scope,
        legacyScope,
        changeDefaultLanguage = true,
    }: {
        scope: ETranslationScope
        legacyScope?: ETranslationScope
        changeDefaultLanguage?: boolean
    }
): boolean => {
    const previousOverrides = appliedTranslationOverrides.get(scope) ?? []
    previousOverrides
        .slice()
        .reverse()
        .forEach(({key, language, previousValue}) => {
            // Unwind in reverse so a child key is removed before its parent
            // object is restored. i18next treats undefined as deletion; its
            // public type only exposes string values, while getResource can
            // also return objects or undefined from the base resource layer.
            i18n.addResource(language, "translations", key, previousValue as string, {
                silent: true,
            })
        })
    appliedTranslationOverrides.delete(scope)

    const i18nObj = filterTranslationOverrides(config?.i18n, scope, legacyScope)
    const nextOverrides: IAppliedTranslationOverride[] = []

    Object.entries(i18nObj ?? {}).forEach(([language, translations]) => {
        Object.entries(translations).forEach(([key, value]) => {
            nextOverrides.push({
                key,
                language,
                previousValue: cloneTranslationResource(
                    i18n.getResource(language, "translations", key)
                ),
            })
            i18n.addResource(language, "translations", key, value, {silent: true})
        })
    })
    if (nextOverrides.length > 0) {
        appliedTranslationOverrides.set(scope, nextOverrides)
    }

    // Both adding and restoring resources must update already-mounted React
    // consumers. Emit once so a replacement is observed atomically.
    if (previousOverrides.length > 0 || nextOverrides.length > 0) {
        i18n.emit("languageChanged", i18n.language)
    }

    if (changeDefaultLanguage) {
        // Apply language policy: skip if query param provided, otherwise check for FORCE_DEFAULT
        return applyConfigurationLanguagePolicy(config)
    }
    return false
}

export default i18n
