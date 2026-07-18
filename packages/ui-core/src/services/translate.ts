// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

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

const getPrimaryLanguageCode = (lang: string): string => lang.split("-")[0].toLowerCase()

export const translateFromPresentation = (
    object: any,
    key: string,
    lang: string
): string | undefined => {
    const presentation = object?.["i18n"] ? object : object?.["presentation"]

    if (!presentation?.["i18n"]) {
        return object?.[key] || undefined
    }

    const userLanguage = getPrimaryLanguageCode(lang)
    const defaultLanguageCode = presentation["language_conf"]?.["default_language_code"]
    const defaultLanguage = defaultLanguageCode
        ? getPrimaryLanguageCode(defaultLanguageCode)
        : undefined

    return (
        presentation["i18n"][userLanguage]?.[key] ||
        (defaultLanguage ? presentation["i18n"][defaultLanguage]?.[key] : undefined) ||
        undefined
    )
}
