export const translate = (object: any, key: string, lang: string): string | undefined => {
    if (object && object[key]) {
        if (object[`${key}_i18n`]) {
            return object[`${key}_i18n`][lang] || object[key]["en"]
        }
        return object[key]
    }
    return undefined
}

export const translateElection = (object: any, key: string, lang: string): string => {
    if (object && object["presentation"] && object["presentation"]["i18n"]) {
        return object["presentation"]["i18n"][lang][key] || object[key]
    } else {
        return object[key]
    }
}
