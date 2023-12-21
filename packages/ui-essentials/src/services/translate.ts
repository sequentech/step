type TranslationDict = {[lang: string]: string}

export const translate = <T, K extends keyof T>(
    input: T,
    key: K,
    lang: string
): string | undefined => {
    const i18n_key = `${String(key)}_i18n`
    if ((input as any)[i18n_key]) {
        let dict = (input as any)[i18n_key] as TranslationDict

        if (lang in dict) {
            return dict[lang]
        }
    }

    return input[key] as string
}

export const translateElection = (object: any, key: string, lang: string): string => {
    if (object && object["presentation"] && object["presentation"]["i18n"]) {
        return object["presentation"]["i18n"][lang][key] || object[key]
    } else {
        return object[key]
    }
}
