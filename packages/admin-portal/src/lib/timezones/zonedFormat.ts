// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Admin Portal times with their zone (VOTE-LIFECYCLE): which zone a value is
 * shown in, and the "one value with a label" and "my time" texts. The zone
 * texts come from the `timezones.*` keys through ui-core; nothing here puts a
 * zone label together itself.
 */
import type {TFunction} from "i18next"
import {
    browserTimeZone,
    canonicalZone,
    DEFAULT_TIME_ZONE,
    effectiveTimeZone,
    ELogTimeZonePolicy,
    formatDateTimeZone,
    formatMyTime,
    primaryTimeZone,
    type IElectionEventPresentation,
    type IElectionPresentation,
} from "@sequentech/ui-core"

export {DEFAULT_TIME_ZONE, effectiveTimeZone, primaryTimeZone}

export type DateValue = string | number | Date | null | undefined

/** A date from an ISO string, a `Date` or Unix milliseconds; null when it isn't one. */
export const toDate = (value: DateValue): Date | null => {
    if (value === null || value === undefined || value === "") return null
    const date = value instanceof Date ? value : new Date(value)
    return Number.isNaN(date.getTime()) ? null : date
}

/**
 * The zone the Logs tab shows a row of `election` in: the primary under the
 * PRIMARY policy, else the row's election zone (sequent-core `log_time_zone`).
 */
export const logTimeZone = (
    event?: IElectionEventPresentation | null,
    election?: IElectionPresentation | null
): string =>
    (event?.timezones?.logs ?? ELogTimeZonePolicy.ELECTION) === ELogTimeZonePolicy.PRIMARY
        ? primaryTimeZone(event)
        : effectiveTimeZone(event, election)

export interface IZonedFormatOptions {
    t: TFunction
    lang: string
    /** Show seconds (logs, tasks). */
    seconds?: boolean
}

/**
 * The date and time part in `zone`, in the i18n language's format: the
 * timezone service's default (medium date, short time), with seconds for
 * logs and tasks.
 */
export const formatWallTime = (date: Date, zone: string, lang: string, seconds = false): string => {
    const locale = intlLocale(lang)
    const options: Intl.DateTimeFormatOptions = {
        dateStyle: "medium",
        timeStyle: seconds ? "medium" : "short",
    }
    try {
        return new Intl.DateTimeFormat(locale, {...options, timeZone: zone}).format(date)
    } catch {
        // A zone name the browser doesn't know: show it in UTC rather than
        // in the browser's zone, as the server does.
        return new Intl.DateTimeFormat(locale, {...options, timeZone: DEFAULT_TIME_ZONE}).format(
            date
        )
    }
}

/** The Intl locale of an i18n language: Catalan is "cat" in the portals. */
const intlLocale = (lang: string): string => {
    const locale = lang.split("-")[0] === "cat" ? "ca" : lang
    try {
        return Intl.getCanonicalLocales(locale)[0] ?? "en"
    } catch {
        return "en"
    }
}

/** "Apr 9, 2028, 3:41:15 AM PhST" (timezones.dateTimeZone). */
export const formatZoned = (
    value: DateValue,
    zone: string,
    {t, lang, seconds}: IZonedFormatOptions
): string => {
    const date = toDate(value)
    if (!date) return ""
    return formatDateTimeZone(date, zone, {
        t,
        lang,
        formatDateTime: (instant, inZone) => formatWallTime(instant, inZone, lang, seconds),
    })
}

/** The same zone, so a "my time" line would repeat the first. */
export const sameZone = (left: string, right: string): boolean =>
    canonicalZone(left) === canonicalZone(right)

/**
 * "Apr 8, 2028, 6:00:03 PM EDT · my time" (timezones.myTime), in the
 * browser's zone; null when the value is already shown in it.
 */
export const formatMine = (
    value: DateValue,
    zone: string,
    {t, lang, seconds}: IZonedFormatOptions,
    myZone: string = browserTimeZone()
): string | null => {
    const date = toDate(value)
    if (!date || sameZone(zone, myZone)) return null
    return formatMyTime(
        date,
        {
            t,
            lang,
            formatDateTime: (instant, inZone) => formatWallTime(instant, inZone, lang, seconds),
        },
        myZone
    )
}
