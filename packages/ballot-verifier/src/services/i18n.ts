// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {initializeLanguages} from "@sequentech/ui-core"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import catalanTranslation from "../translations/cat"
import frenchTranslation from "../translations/fr"
import tagalogTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"

initializeLanguages({
    en: englishTranslation,
    es: spanishTranslation,
    cat: catalanTranslation,
    fr: frenchTranslation,
    tl: tagalogTranslation,
    gl: galegoTranslation,
})
