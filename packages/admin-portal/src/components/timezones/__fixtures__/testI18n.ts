// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// An i18next instance for the timezone tests: the ui-core and admin English
// texts merged as the portal merges them, plus optional overrides.
import i18next, {type i18n as I18n} from "i18next"
import merge from "lodash/merge"
import englishTranslation from "@/translations/en"
import uiCoreEnglish from "../../../../../ui-core/src/translations/en"

export {MY_TIME_ZONE} from "./configurations"

export const testI18n = (overrides: Record<string, unknown> = {}): I18n => {
    const instance = i18next.createInstance()
    void instance.init({
        lng: "en",
        fallbackLng: "en",
        initAsync: false,
        resources: {
            en: {
                translations: merge(
                    {},
                    uiCoreEnglish.translations,
                    englishTranslation.translations,
                    overrides
                ),
            },
        },
        ns: ["translations"],
        defaultNS: "translations",
        interpolation: {escapeValue: false},
    })
    return instance
}
