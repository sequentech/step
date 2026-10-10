// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useMemo, type PropsWithChildren} from "react"
import {useTranslation} from "react-i18next"
import {
    browserTimeZone,
    canonicalZone,
    formatDateTimeZone,
    formatMyTime,
    formatPlaceTime,
    instantToZoned,
    listTimeZones,
    searchTimeZones,
    timeZoneOption,
    timeZonePrimaryOptionLabel,
    zoneCity,
    zoneLabel,
    zoneName,
    zoneOffsetMinutes,
    zonedTimeNote,
    zonedToInstant,
    type ITimeZoneTextOptions,
} from "@sequentech/ui-core"

/**
 * The viewer's zone, "my time": the browser's, unless a story or test fixes
 * it, so what they show never depends on the machine they run on.
 */
const MyTimeZoneContext = createContext<string | null>(null)

export const MyTimeZoneProvider: React.FC<PropsWithChildren<{zone: string}>> = ({
    zone,
    children,
}) => <MyTimeZoneContext.Provider value={zone}>{children}</MyTimeZoneContext.Provider>

/**
 * `{{dateTime}}` in admin screens: the i18n language's medium date and a
 * 24-hour time, as the wall time in `zone`. The zone itself is never part of
 * the pattern; the `timezones.*` strings place it.
 */
export const adminDateTimeFormat = (language: string) => {
    const locale = language.split("-")[0] === "cat" ? "ca" : language
    return (instant: Date, zone: string): string => {
        const options: Intl.DateTimeFormatOptions = {
            day: "2-digit",
            month: "short",
            year: "numeric",
            hour: "2-digit",
            minute: "2-digit",
            hourCycle: "h23",
            timeZone: zone,
        }
        try {
            return new Intl.DateTimeFormat(locale, options).format(instant)
        } catch {
            // A language Intl doesn't know (e.g. a custom code): the default locale.
            return new Intl.DateTimeFormat(undefined, options).format(instant)
        }
    }
}

/**
 * A wall time (`YYYY-MM-DDTHH:MM`) in the admin format as written, without
 * resolving it in a zone, so a time that doesn't exist (a DST gap) can be named.
 */
export const formatWallTime = (local: string, options: ITimeZoneTextOptions): string => {
    const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(local)
    if (!match) return local
    const [, year, month, day, hour, minute] = match.map(Number)
    const asUtc = new Date(Date.UTC(year, month - 1, day, hour, minute))
    asUtc.setUTCFullYear(year)
    return (options.formatDateTime ?? adminDateTimeFormat(options.lang))(asUtc, "UTC")
}

/** The ui-core timezone service (design §3) with the admin's texts and the viewer's zone. */
export interface IAdminTimeZones {
    listTimeZones: typeof listTimeZones
    canonicalZone: typeof canonicalZone
    searchTimeZones: typeof searchTimeZones
    timeZoneOption: typeof timeZoneOption
    timeZonePrimaryOptionLabel: typeof timeZonePrimaryOptionLabel
    zoneOffsetMinutes: typeof zoneOffsetMinutes
    zonedToInstant: typeof zonedToInstant
    instantToZoned: typeof instantToZoned
    zoneLabel: typeof zoneLabel
    zoneName: typeof zoneName
    zoneCity: typeof zoneCity
    formatDateTimeZone: typeof formatDateTimeZone
    formatPlaceTime: typeof formatPlaceTime
    zonedTimeNote: typeof zonedTimeNote
    /** The viewer's zone. */
    myTimeZone: string
    /** `{{dateTime}} {{zone}} · my time`, in the viewer's zone. */
    formatMyTime: (instant: Date | string) => string
    /** The text options every call takes: t, the language and the admin format. */
    text: ITimeZoneTextOptions
}

/** The timezone service with the admin's texts bound. */
export const useTimeZoneService = (): IAdminTimeZones => {
    const fixed = useContext(MyTimeZoneContext)
    const {t, i18n} = useTranslation()
    const language = i18n?.language ?? "en"
    return useMemo(() => {
        const text: ITimeZoneTextOptions = {
            t,
            lang: language,
            formatDateTime: adminDateTimeFormat(language),
        }
        const myTimeZone = fixed ?? browserTimeZone()
        return {
            listTimeZones,
            canonicalZone,
            searchTimeZones,
            timeZoneOption,
            timeZonePrimaryOptionLabel,
            zoneOffsetMinutes,
            zonedToInstant,
            instantToZoned,
            zoneLabel,
            zoneName,
            zoneCity,
            formatDateTimeZone,
            formatPlaceTime,
            zonedTimeNote,
            myTimeZone,
            formatMyTime: (instant: Date | string) => formatMyTime(instant, text, myTimeZone),
            text,
        }
    }, [fixed, t, language])
}
