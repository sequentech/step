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

/**
 * A fraction as a percentage, rounded half away from zero, with
 * `sequent_core::monitoring::compute::percent_label`'s rule: never all short of
 * all, nor none above none. 9,996 of 10,000 Posts closed is 99.9%, not 100.0%.
 */
function percent(value: number, locale: string, minimumDigits: number, maximumDigits: number) {
    const format = new Intl.NumberFormat(locale, {
        style: "percent",
        minimumFractionDigits: minimumDigits,
        maximumFractionDigits: maximumDigits,
    }).format
    const text = format(value)
    // The smallest step the text shows, as a fraction.
    const step = 10 ** -(maximumDigits + 2)
    if (value < 1 && text === format(1)) return format(1 - step)
    if (value > 0 && text === format(0)) return format(step)
    return text
}

/** A fraction as a percentage, `53.2%`, for KPIs. */
export function formatRatio(value: unknown, locale: string, digits = 1): string {
    return isNumber(value) ? percent(value, locale, digits, digits) : UNDEFINED_VALUE
}

/** Digits a table shows of a ratio: enough to tell 99.96% from 100%. */
const TABLE_RATIO_DIGITS = 4

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
            return isNumber(value) ? percent(value, locale, 0, TABLE_RATIO_DIGITS) : UNDEFINED_VALUE
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
