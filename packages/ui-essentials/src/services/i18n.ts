// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import i18n, {i18n as I18N, Resource} from "i18next"
import {deepmerge} from "@mui/utils"
import LanguageDetector from "i18next-browser-languagedetector"
import {initReactI18next} from "react-i18next"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import catalanTranslation from "../translations/cat"
import frenchTranslation from "../translations/fr"

export const initializeLanguages = (externalTranslations: Resource, language: string = "en") => {
    const libTranslations: Resource = {
        en: englishTranslation,
        es: spanishTranslation,
        cat: catalanTranslation,
        fr: frenchTranslation,
    }
    const mergedTranslations = deepmerge(libTranslations, externalTranslations)

    i18n.use(LanguageDetector)
        .use(initReactI18next)
        .init({
            // we init with resources
            resources: mergedTranslations,
            fallbackLng: "en",
            lng: language, // Use provided language or fallback to english if not available
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
        })
}

export const getLanguages = (i18n: I18N) => Object.keys(i18n.services.resourceStore.data)

export default i18n
