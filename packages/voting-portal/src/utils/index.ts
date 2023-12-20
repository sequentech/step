export const translate = (object: any, key: string, lang: string): string | undefined => {
    console.log("translate object", object)

    if (object && object[key]) {
        if (object[`${key}_i18n`]) {
            return object[key][lang] || object[key]["en"]
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
