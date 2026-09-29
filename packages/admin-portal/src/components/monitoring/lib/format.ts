// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EColumnKind} from "../types"

/** Shown for a figure that is undefined, such as a ratio over zero. */
export const UNDEFINED_VALUE = "—"

const isNumber = (value: unknown): value is number =>
    typeof value === "number" && Number.isFinite(value)

export function formatInteger(value: unknown, locale: string): string {
    return isNumber(value) ? new Intl.NumberFormat(locale).format(value) : UNDEFINED_VALUE
}

/** A fraction as a percentage, `53.2%`, for KPIs. */
export function formatRatio(value: unknown, locale: string, digits = 1): string {
    return isNumber(value)
        ? new Intl.NumberFormat(locale, {
              style: "percent",
              minimumFractionDigits: digits,
              maximumFractionDigits: digits,
          }).format(value)
        : UNDEFINED_VALUE
}

/** `647K`, as on charts. */
export function formatCompact(value: unknown, locale: string): string {
    return isNumber(value)
        ? new Intl.NumberFormat(locale, {notation: "compact", maximumFractionDigits: 1}).format(
              value
          )
        : UNDEFINED_VALUE
}

/** A result cell, exact enough to check a chart against. */
export function formatCell(value: unknown, kind: string, locale: string): string {
    if (value === null || value === undefined) return UNDEFINED_VALUE
    switch (kind) {
        case EColumnKind.INTEGER:
            return formatInteger(value, locale)
        case EColumnKind.NUMBER:
            return isNumber(value)
                ? new Intl.NumberFormat(locale, {
                      style: "percent",
                      maximumFractionDigits: 2,
                  }).format(value)
                : UNDEFINED_VALUE
        default:
            return typeof value === "string" ? value : JSON.stringify(value)
    }
}

function dateTimeFormat(
    locale: string,
    timeZone: string | undefined,
    options: Intl.DateTimeFormatOptions
) {
    try {
        return new Intl.DateTimeFormat(locale, {...options, timeZone})
    } catch {
        return new Intl.DateTimeFormat(locale, options)
    }
}

/** A moment in the event's time zone; the viewer's zone if that one is unknown. */
export function formatDateTime(
    iso: string | null | undefined,
    timeZone: string | undefined,
    locale: string,
    options: Intl.DateTimeFormatOptions = {dateStyle: "medium", timeStyle: "short"}
): string {
    const date = iso ? new Date(iso) : null
    if (!date || Number.isNaN(date.getTime())) return UNDEFINED_VALUE
    return dateTimeFormat(locale, timeZone, options).format(date)
}
