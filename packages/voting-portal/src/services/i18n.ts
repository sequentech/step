// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {initializeLanguages} from "@sequentech/ui-essentials"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import hebrewTranslation from "../translations/he"

initializeLanguages({
    en: englishTranslation,
    es: spanishTranslation,
    he: hebrewTranslation,
})
