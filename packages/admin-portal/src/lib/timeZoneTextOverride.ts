// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {missingTimeZonePlaceholders} from "@sequentech/ui-core"
import {TFunction} from "i18next"

/**
 * Why the Localization tabs refuse an override, if they do: a combined
 * timezone string (`timezones.dateTimeZone`, `myTime`, `placeTime`,
 * `voterDateTimeZone`, `onThisDevice`, `gap`, `overlap`) must keep
 * `{{dateTime}}` and its zone placeholder in every scope.
 */
export const timeZoneTextOverrideError = (
    t: TFunction,
    key: string,
    value: string
): string | undefined => {
    const missing = missingTimeZonePlaceholders(key, value)
    if (missing.length === 0) {
        return undefined
    }
    return String(
        t("electionEventScreen.localization.notify.invalidTimeZoneText", {
            placeholders: missing.map((name: string) => `{{${name}}}`).join(", "),
            interpolation: {escapeValue: false},
        })
    )
}
