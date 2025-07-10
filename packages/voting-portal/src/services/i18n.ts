// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {initializeLanguages} from "@sequentech/ui-core"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import catalanTranslation from "../translations/cat"
import frenchTranslation from "../translations/fr"
import tagalotTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"
import dutchTranslation from "../translations/nl"
import basqueTranslation from "../translations/eu"
import {getLanguageFromURL} from "../utils/queryParams"

const language = getLanguageFromURL()

initializeLanguages(
    {
        en: englishTranslation,
        es: spanishTranslation,
        cat: catalanTranslation,
        fr: frenchTranslation,
        tl: tagalotTranslation,
        gl: galegoTranslation,
        nl: dutchTranslation,
        eu: basqueTranslation,
    },
    language
)
