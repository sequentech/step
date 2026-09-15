// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETranslationScope, getActiveTranslationScope} from "./translationScopes"

export type TranslationDict = {[lang: string]: string}

export const translate = <T, K extends keyof T>(
    input: T,
    key: K,
    lang: string
): string | undefined => {
    const i18n_key = `${String(key)}_i18n`
    if ((input as any)?.[i18n_key]) {
        let dict = (input as any)[i18n_key] as TranslationDict

        if (lang in dict) {
            return dict[lang]
        }
    }

    return input[key] as string
}

type TranslationValue = string | null | undefined

interface TranslatablePresentation {
    i18n?: Record<string, Record<string, TranslationValue>>
}

type TranslatableEntity<K extends string = string> = Partial<
    Record<K, string | null | undefined>
> & {
    presentation?: TranslatablePresentation | null
}

type TranslationInput<K extends string = string> =
    | TranslatablePresentation
    | TranslatableEntity<K>
    | null
    | undefined

interface TranslateFromPresentationOptions {
    defaultLanguageCode?: string
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

export const isTranslatablePresentation = (value: unknown): value is TranslatablePresentation => {
    if (!isRecord(value)) {
        return false
    }

    if (value.i18n === undefined) {
        return true
    }

    if (!isRecord(value.i18n)) {
        return false
    }

    return Object.values(value.i18n).every(
        (translations) =>
            isRecord(translations) &&
            Object.values(translations).every(
                (translation) => translation == null || typeof translation === "string"
            )
    )
}

const getPresentation = <K extends string>(
    object: TranslationInput<K>
): TranslatablePresentation | undefined => {
    if (!isRecord(object)) {
        return undefined
    }

    const value =
        "i18n" in object && object.i18n
            ? object
            : "presentation" in object
              ? object.presentation
              : object
    return isRecord(value) ? (value as TranslatablePresentation) : undefined
}

const getPrimaryLanguageCode = (lang?: string): string | undefined =>
    lang?.split("-")[0].toLowerCase()

const getTranslatedValue = (
    presentation: TranslatablePresentation | undefined,
    language: string | undefined,
    key: string
): string | undefined => {
    if (!language) {
        return undefined
    }

    const translations = presentation?.i18n?.[language]
    if (!translations) {
        return undefined
    }

    const activeScope = getActiveTranslationScope()
    const candidateKeys = [
        ...(activeScope && activeScope !== ETranslationScope.GLOBAL
            ? [`${activeScope}:${key}`]
            : []),
        `${ETranslationScope.GLOBAL}:${key}`,
        key,
    ]

    for (const candidateKey of candidateKeys) {
        const value = translations[candidateKey]
        if (typeof value === "string" && value.length > 0) {
            return value
        }
    }
    return undefined
}

export const translateFromPresentation = <K extends string>(
    object: TranslationInput<K>,
    key: K,
    lang: string,
    options: TranslateFromPresentationOptions = {}
): string | undefined => {
    const presentation = getPresentation(object)
    const userLanguage = getPrimaryLanguageCode(lang)
    const defaultLanguage = getPrimaryLanguageCode(options.defaultLanguageCode)

    const translatedValue =
        getTranslatedValue(presentation, userLanguage, key) ||
        getTranslatedValue(presentation, defaultLanguage, key)

    if (translatedValue) {
        return translatedValue
    }

    if (isRecord(object) && "i18n" in object && Boolean(object.i18n)) {
        return undefined
    }

    if (!isRecord(object)) {
        return undefined
    }

    const legacyValue = object[key]
    return typeof legacyValue === "string" && legacyValue.length ? legacyValue : undefined
}
