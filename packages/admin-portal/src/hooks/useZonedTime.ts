// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback} from "react"
import {useTranslation} from "react-i18next"
import {formatDateTimeZone} from "@sequentech/ui-core"
import {useEventZonedFormat} from "@/hooks/useZonedFormat"

/** The Intl locale of an i18n language: Catalan is "cat" in the portals. */
export const intlLanguage = (lang: string): string => (lang.split("-")[0] === "cat" ? "ca" : lang)

const format = (
    instant: Date,
    zone: string,
    lang: string,
    options: Intl.DateTimeFormatOptions
): string => {
    try {
        return new Intl.DateTimeFormat(lang, {...options, timeZone: zone}).format(instant)
    } catch {
        return new Intl.DateTimeFormat(lang, {...options, timeZone: "UTC"}).format(instant)
    }
}

/** The calendar day of `instant` in `zone`, to compare two instants. */
const dayIn = (instant: Date, zone: string): string =>
    format(instant, zone, "en-CA", {year: "numeric", month: "2-digit", day: "2-digit"})

/**
 * "6:15 PM CET": a time in the event's timezone (the election's on its
 * screens), labelled through the `timezones.*` texts. A time on another day
 * than `now` in that zone also shows its date: "Mar 13, 2028, 6:15 PM CET".
 */
export const useZonedTime = (now?: Date): ((value?: string | null) => string) => {
    const {t, i18n} = useTranslation()
    const {zone} = useEventZonedFormat()
    const language = i18n?.language ?? "en"
    const nowTime = now?.getTime()
    return useCallback(
        (value?: string | null) => {
            if (!value) return ""
            const today = nowTime === undefined ? new Date() : new Date(nowTime)
            return formatDateTimeZone(value, zone, {
                t,
                lang: language,
                formatDateTime: (instant, inZone) =>
                    format(
                        instant,
                        inZone,
                        intlLanguage(language),
                        dayIn(instant, inZone) === dayIn(today, inZone)
                            ? {timeStyle: "short"}
                            : {dateStyle: "medium", timeStyle: "short"}
                    ),
            })
        },
        [t, language, zone, nowTime]
    )
}
