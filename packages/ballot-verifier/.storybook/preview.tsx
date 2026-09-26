// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Preview} from "@storybook/react-vite"
import preview from "../../ui-essentials/.storybook/preview"
import {DEFAULT_STORY_GLOBALS, storyGlobalTypes} from "../../ui-essentials/.storybook/globals"
import {initializeLanguages} from "@sequentech/ui-core"
import englishTranslation from "../src/translations/en"
import spanishTranslation from "../src/translations/es"
import catalanTranslation from "../src/translations/cat"
import frenchTranslation from "../src/translations/fr"
import tagalogTranslation from "../src/translations/tl"
import galegoTranslation from "../src/translations/gl"

// The verifier ships no Dutch or Basque strings; ui-core's cover the shared components.
initializeLanguages(
    {
        en: englishTranslation,
        es: spanishTranslation,
        cat: catalanTranslation,
        fr: frenchTranslation,
        tl: tagalogTranslation,
        gl: galegoTranslation,
    },
    DEFAULT_STORY_GLOBALS.locale
)

export default {
    ...preview,
    globalTypes: {...preview.globalTypes, tenant: storyGlobalTypes.tenant},
} satisfies Preview
