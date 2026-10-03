// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMemo} from "react"
import {useTranslation} from "react-i18next"
import type {TFunction} from "i18next"
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import {colonHex} from "@/lib/signing/der"
import {DocumentKind, type SigningAction} from "@/lib/signing/types"
import type {ISigningPanelData} from "@/lib/signing/api"

const parse = (value: string | Date | null | undefined): Date | null => {
    if (!value) return null
    const date = value instanceof Date ? value : new Date(value)
    return Number.isNaN(date.getTime()) ? null : date
}

const formatter = (
    locale: string,
    options: Intl.DateTimeFormatOptions,
    timeZone: string | null | undefined
): Intl.DateTimeFormat => {
    try {
        return new Intl.DateTimeFormat(locale, {...options, timeZone: timeZone || undefined})
    } catch {
        // An unknown zone name: the browser's own zone.
        return new Intl.DateTimeFormat(locale, options)
    }
}

/** "19:11 GMT+8": 24-hour time with the zone, as signers compare times. */
export const formatTime = (
    value: string | Date | null | undefined,
    locale: string,
    timeZone?: string | null
): string => {
    const date = parse(value)
    return date
        ? formatter(
              locale,
              {hour: "2-digit", minute: "2-digit", hourCycle: "h23", timeZoneName: "short"},
              timeZone
          ).format(date)
        : ""
}

/** "Jan 11, 2030" in English. */
export const formatDate = (
    value: string | Date | null | undefined,
    locale: string,
    timeZone?: string | null
): string => {
    const date = parse(value)
    return date
        ? formatter(locale, {day: "numeric", month: "short", year: "numeric"}, timeZone).format(
              date
          )
        : ""
}

/** Times and dates in the election event's zone (the panel's `time_zone`), else the browser's. */
export const useSigningFormat = (timeZone?: string | null) => {
    const {i18n} = useTranslation()
    const locale = i18n.language
    return useMemo(
        () => ({
            time: (value: string | Date | null | undefined) => formatTime(value, locale, timeZone),
            date: (value: string | Date | null | undefined) => formatDate(value, locale, timeZone),
        }),
        [locale, timeZone]
    )
}

/**
 * The organization's name in copy: the tenant's display name
 * (`settings.display_name`), else its slug, which the menu shows.
 */
export const organizationName = (
    tenant: Sequent_Backend_Tenant | null | undefined
): string | null => {
    const displayName = (tenant?.settings as {display_name?: unknown} | null | undefined)
        ?.display_name
    if (typeof displayName === "string" && displayName.trim()) {
        return displayName.trim()
    }
    return tenant?.slug || null
}

/** "Jose R. Dela Cruz and Ana P. Reyes". */
export const formatList = (names: string[], locale: string): string =>
    new Intl.ListFormat(locale, {style: "long", type: "conjunction"}).format(names)

/** "7f3a91c2…0de04b": enough of a SHA-256 to compare by eye. */
export const shortHash = (lowerHex: string): string =>
    lowerHex.length > 16 ? `${lowerHex.slice(0, 8)}…${lowerHex.slice(-6)}` : lowerHex

/** "F7:08:0F:24:0A:58:E1:B6…D6:FE:78". */
export const shortFingerprint = (lowerHex: string): string => {
    const full = colonHex(lowerHex)
    return full.length > 36 ? `${full.slice(0, 23)}…${full.slice(-8)}` : full
}

export const documentTypeLabel = (kind: DocumentKind): string =>
    kind === DocumentKind.Eml ? "EML" : "PDF"

/** "Election returns · Madrid PE · Spain": the action, then the Post and the country. */
export const requestTitle = (t: TFunction, data: ISigningPanelData): string =>
    [t(`signing.actions.${data.request.action}.short`), data.election_name, data.area_name]
        .filter((part): part is string => !!part)
        .join(" · ")

export const actionObject = (t: TFunction, action: SigningAction): string =>
    t(`signing.actions.${action}.object`)
