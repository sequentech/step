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
import {ELanguageDetectionPolicy, ILanguageConf} from "../types/LanguageConf"
import {getValueFromCookie} from "../utils/cookies"
import {iso_639_2t_to_bcp47_js, locale_to_internal_language_code_js} from "sequent-core"
import {
    ETranslationScope,
    filterTranslationOverrides,
    setActiveTranslationScope,
} from "./translationScopes"
import {INFORMAL_SUFFIX, splitRegister, withRegister, withRegisterBCP47} from "./registerLocale"

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

const PLURAL_SUFFIXES = ["zero", "one", "two", "few", "many", "other"]

/**
 * Overrides stored before a key was split into plural forms target the bare
 * key, but when `count` is passed i18next looks up `key_<form>` first, so the
 * bundled plural forms would shadow them. Copy such an override onto every
 * plural form the bundle defines, unless the override sets that form itself.
 */
const expandUnsuffixedPluralOverrides = (
    language: string,
    translations: Record<string, string>
): Record<string, string> => {
    const expanded: Record<string, string> = {}

    Object.entries(translations).forEach(([key, value]) => {
        PLURAL_SUFFIXES.forEach((suffix) => {
            const pluralKey = `${key}_${suffix}`
            if (
                !(pluralKey in translations) &&
                i18n.getResource(language, "translations", pluralKey) !== undefined
            ) {
                expanded[pluralKey] = value
            }
        })
        expanded[key] = value
    })

    return expanded
}

const applyTranslationOverrides = (
    overrides: Record<string, Record<string, string>> | undefined
): IAppliedTranslationOverride[] => {
    const appliedOverrides: IAppliedTranslationOverride[] = []

    Object.entries(overrides ?? {}).forEach(([language, translations]) => {
        const expanded = expandUnsuffixedPluralOverrides(language, translations)
        Object.entries(expanded).forEach(([key, value]) => {
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
        // WASM not yet initialised. Returning `lang` unchanged would put the
        // internal code straight into `<html lang>`, and `es-tu` is not
        // well-formed BCP 47 — `tu` is neither a registered variant nor behind
        // the private-use singleton. `languageChanged` only repairs it if the
        // voter switches language, which most never do, so compose a valid tag
        // from the base instead.
        return withRegisterBCP47(base, informal)
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
            // React escapes string children, so values need no escaping here.
            // Translations rendered as HTML are the exception: they escape their
            // own values through translateHtml/escapeTranslationValues.
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

            const expanded = expandUnsuffixedPluralOverrides(language, translations)
            Object.entries(expanded).forEach(([key, value]) => {
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
