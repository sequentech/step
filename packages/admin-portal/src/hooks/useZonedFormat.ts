// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMemo} from "react"
import {useTranslation} from "react-i18next"
import type {IElectionEventPresentation} from "@sequentech/ui-core"
import {useMyTimeZone, useZonedEvent} from "@/providers/EventTimeZoneProvider"
import {formatMine, formatZoned, primaryTimeZone, type DateValue} from "@/lib/timezones/zonedFormat"

export interface IZonedFormat {
    /** The zone values are shown in (IANA). */
    zone: string
    /** The viewer's zone ("my time"). */
    myZone: string
    /** One value with a label: "Apr 9, 2028, 3:41:15 AM PhST". */
    format: (value: DateValue, zone?: string) => string
    /** The browser's time ("… EDT · my time"), or null when it's the same zone. */
    mine: (value: DateValue, zone?: string) => string | null
}

/**
 * Times in `zone` (the viewer's when absent), labelled through the
 * `timezones.*` keys, so translation overrides change them. `format` and
 * `mine` take another zone for rows with their own (log rows).
 */
export const useZonedFormat = (
    zone?: string | null,
    {seconds = false}: {seconds?: boolean} = {}
): IZonedFormat => {
    const {t, i18n} = useTranslation()
    const lang = i18n.language
    const myZone = useMyTimeZone()
    return useMemo(() => {
        const shown = zone || myZone
        const options = {t, lang, seconds}
        return {
            zone: shown,
            myZone,
            format: (value, inZone) => formatZoned(value, inZone || shown, options),
            mine: (value, inZone) => formatMine(value, inZone || shown, options, myZone),
        }
        // `t` changes when the language or the loaded resources do.
    }, [zone, myZone, t, lang, seconds])
}

/** An election event, or its id, as the screens showing its times know it. */
export type ZonedEventRef = {id?: unknown; presentation?: unknown} | string | null | undefined

/**
 * The presentation of `event`: the record when the caller has it, else the
 * event of the screen (EventTimeZoneProvider) when it is that event or no
 * event is named. Nothing is read from the server.
 */
export const useEventPresentation = (
    event?: ZonedEventRef
): IElectionEventPresentation | undefined => {
    const screenEvent = useZonedEvent()
    if (event && typeof event === "object" && "presentation" in event) {
        return (event.presentation ?? undefined) as IElectionEventPresentation | undefined
    }
    const id = event && typeof event === "object" ? event.id : event
    if (screenEvent && (!id || String(id) === screenEvent.id)) {
        return screenEvent.presentation ?? undefined
    }
    return undefined
}

/**
 * Admin times of an election event: in its primary zone (on an election's
 * screens, the election's zone). Outside the event's screens, the viewer's.
 */
export const useEventZonedFormat = (
    event?: ZonedEventRef,
    options?: {seconds?: boolean}
): IZonedFormat => {
    const screenEvent = useZonedEvent()
    const presentation = useEventPresentation(event)
    const id = event && typeof event === "object" ? event.id : event
    const isScreenEvent =
        !!screenEvent &&
        !(event && typeof event === "object" && "presentation" in event) &&
        (!id || String(id) === screenEvent.id)
    const zone = isScreenEvent
        ? screenEvent.zone
        : presentation
          ? primaryTimeZone(presentation)
          : null
    return useZonedFormat(zone, options)
}
