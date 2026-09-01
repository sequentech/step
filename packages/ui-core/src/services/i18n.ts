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
import {
    ETranslationScope,
    filterTranslationOverrides,
    setActiveTranslationScope,
} from "./translationScopes"

export const USER_LANGUAGE_COOKIE_NAME = "USER_LANGUAGE"

interface IAppliedTranslationOverride {
    key: string
    language: string
    previousValue: unknown
    value: string
}

const appliedTranslationOverrides = new Map<ETranslationScope, IAppliedTranslationOverride[]>()

const cloneTranslationResource = (value: unknown): unknown =>
    typeof value === "object" && value !== null ? deepmerge({}, value) : value

const restoreTranslationOverrides = (overrides: IAppliedTranslationOverride[]) => {
    overrides
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
}

const reapplyTranslationOverrides = (
    overrides: IAppliedTranslationOverride[]
): IAppliedTranslationOverride[] =>
    overrides.map(({key, language, value}) => {
        const reappliedOverride = {
            key,
            language,
            previousValue: cloneTranslationResource(
                i18n.getResource(language, "translations", key)
            ),
            value,
        }
        i18n.addResource(language, "translations", key, value, {silent: true})
        return reappliedOverride
    })

const applyTranslationOverrides = (
    overrides: Record<string, Record<string, string>> | undefined
): IAppliedTranslationOverride[] => {
    const appliedOverrides: IAppliedTranslationOverride[] = []

    Object.entries(overrides ?? {}).forEach(([language, translations]) => {
        Object.entries(translations).forEach(([key, value]) => {
            appliedOverrides.push({
                key,
                language,
                previousValue: cloneTranslationResource(
                    i18n.getResource(language, "translations", key)
                ),
                value,
            })
            i18n.addResource(language, "translations", key, value, {silent: true})
        })
    })

    return appliedOverrides
}

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

export const initializeLanguages = (
    externalTranslations: Resource,
    language?: string,
    translationScope?: ETranslationScope
) => {
    setActiveTranslationScope(translationScope)
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

interface IOverwriteTranslationOptions {
    scope: ETranslationScope
    legacyScope?: ETranslationScope
    changeDefaultLanguage?: boolean
}

export function overwriteTranslations(
    config: ITranslationConfiguration | undefined,
    changeDefaultLanguage?: boolean
): boolean
export function overwriteTranslations(
    config: ITranslationConfiguration | undefined,
    options: IOverwriteTranslationOptions
): boolean
export function overwriteTranslations(
    config: ITranslationConfiguration | undefined,
    options: IOverwriteTranslationOptions | boolean = true
): boolean {
    // Preserve the public pre-scoping API for consumers that still pass a
    // boolean (or omit the second argument). Its unprefixed merge semantics
    // remain unchanged; scoped consumers use the options object below.
    if (typeof options === "boolean") {
        const i18nObj = config?.i18n
        if (!i18nObj) {
            return false
        }

        // Legacy writes update the base layer. Temporarily remove scoped
        // overlays, then replay them so they keep their precedence and later
        // cleanup reveals the newly written legacy values.
        const activeOverrideLayers = Array.from(appliedTranslationOverrides.entries())
        activeOverrideLayers
            .slice()
            .reverse()
            .forEach(([, overrides]) => restoreTranslationOverrides(overrides))
        appliedTranslationOverrides.clear()

        Object.entries(i18nObj).forEach(([language, translations]) => {
            const currentResources = i18n.getResourceBundle(language, "translations") || {}
            const nestedTranslations: any = {}

            Object.entries(translations).forEach(([key, value]) => {
                const keys = key.split(".")
                keys.reduce((acc, part, index) => {
                    return (acc[part] = index === keys.length - 1 ? value : acc[part] || {})
                }, nestedTranslations)
            })

            i18n.addResourceBundle(
                language,
                "translations",
                deepmerge(currentResources, nestedTranslations),
                true,
                true
            )
        })

        activeOverrideLayers.forEach(([scope, overrides]) => {
            appliedTranslationOverrides.set(scope, reapplyTranslationOverrides(overrides))
        })
        if (activeOverrideLayers.length > 0) {
            i18n.emit("languageChanged", i18n.language)
        }

        return options ? applyConfigurationLanguagePolicy(config) : false
    }

    const {scope, legacyScope, changeDefaultLanguage = true} = options
    const i18nObj = filterTranslationOverrides(config?.i18n, scope, legacyScope)
    const hasNextOverrides = Object.values(i18nObj ?? {}).some(
        (translations) => Object.keys(translations).length > 0
    )
    const activeOverrideLayers = Array.from(appliedTranslationOverrides.entries())
    const previousLayerIndex = activeOverrideLayers.findIndex(
        ([layerScope]) => layerScope === scope
    )

    if (previousLayerIndex >= 0 || hasNextOverrides) {
        activeOverrideLayers
            .slice()
            .reverse()
            .forEach(([, overrides]) => restoreTranslationOverrides(overrides))
        appliedTranslationOverrides.clear()

        const remainingLayers = activeOverrideLayers.filter(([layerScope]) => layerScope !== scope)
        const insertionIndex = previousLayerIndex >= 0 ? previousLayerIndex : remainingLayers.length

        for (let index = 0; index <= remainingLayers.length; index += 1) {
            if (hasNextOverrides && index === insertionIndex) {
                appliedTranslationOverrides.set(scope, applyTranslationOverrides(i18nObj))
            }

            const remainingLayer = remainingLayers[index]
            if (remainingLayer) {
                const [layerScope, overrides] = remainingLayer
                appliedTranslationOverrides.set(layerScope, reapplyTranslationOverrides(overrides))
            }
        }

        // Emit once after replay so mounted React consumers observe only the
        // final layer order, not the temporary unwind state.
        i18n.emit("languageChanged", i18n.language)
    }

    if (changeDefaultLanguage) {
        // Apply language policy: skip if query param provided, otherwise check for FORCE_DEFAULT
        return applyConfigurationLanguagePolicy(config)
    }
    return false
}

export default i18n
