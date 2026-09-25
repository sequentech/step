// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import preview from "../../ui-essentials/.storybook/preview"
import {initializeLanguages} from "@sequentech/ui-core"
import englishTranslation from "../src/translations/en"
import spanishTranslation from "../src/translations/es"

initializeLanguages({en: englishTranslation, es: spanishTranslation}, "en")

export default {...preview}
