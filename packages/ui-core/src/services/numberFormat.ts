// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ENumberFormatPolicy} from "../types/ElectionEventPresentation"

export const DEFAULT_NUMBER_FORMAT_POLICY = ENumberFormatPolicy.COMMA_PERIOD

export interface INumberSeparators {
    group: string
    decimal: string
}

const NO_BREAK_SPACE = " "
const RIGHT_SINGLE_QUOTATION_MARK = "’"

const SEPARATORS: Record<ENumberFormatPolicy, INumberSeparators> = {
    [ENumberFormatPolicy.COMMA_PERIOD]: {group: ",", decimal: "."},
    [ENumberFormatPolicy.PERIOD_COMMA]: {group: ".", decimal: ","},
    [ENumberFormatPolicy.SPACE_COMMA]: {group: NO_BREAK_SPACE, decimal: ","},
    [ENumberFormatPolicy.SPACE_PERIOD]: {group: NO_BREAK_SPACE, decimal: "."},
    [ENumberFormatPolicy.APOSTROPHE_PERIOD]: {group: RIGHT_SINGLE_QUOTATION_MARK, decimal: "."},
}

export type NumberInput = number | bigint | string | null | undefined

/**
 * The policy an event asks for. Events saved before the policy existed, and
 * values this version does not know, use the default.
 */
export const resolveNumberFormatPolicy = (policy?: string | null): ENumberFormatPolicy =>
    Object.values<string>(ENumberFormatPolicy).includes(policy ?? "")
        ? (policy as ENumberFormatPolicy)
        : DEFAULT_NUMBER_FORMAT_POLICY

export const numberFormatSeparators = (policy?: string | null): INumberSeparators =>
    SEPARATORS[resolveNumberFormatPolicy(policy)]

const groupDigits = (digits: string, separator: string): string =>
    digits.replace(/\B(?=(\d{3})+(?!\d))/g, separator)

const INTEGER = /^-?\d+$/

/**
 * `value` with `decimals` places, its integer part grouped in thousands.
 * Integers given as strings or bigints are grouped digit by digit, so counts
 * beyond 2^53 stay exact. A value that is not a number, such as a `-`
 * placeholder, is returned unchanged.
 */
export const formatNumber = (
    value: NumberInput,
    policy?: string | null,
    decimals: number = 0
): string => {
    if (value === null || value === undefined) {
        return ""
    }
    const {group, decimal} = numberFormatSeparators(policy)
    const text = String(value).trim()
    if (decimals === 0 && INTEGER.test(text)) {
        const negative = text.startsWith("-")
        const digits = (negative ? text.slice(1) : text).replace(/^0+(?=\d)/, "")
        const sign = negative && digits !== "0" ? "-" : ""
        return `${sign}${groupDigits(digits, group)}`
    }
    const numberValue = typeof value === "number" ? value : Number(text)
    if (text === "" || !Number.isFinite(numberValue)) {
        return String(value)
    }
    const fixed = Math.abs(numberValue).toFixed(decimals)
    const [integer, fraction] = fixed.split(".")
    const sign = numberValue < 0 && /[1-9]/.test(fixed) ? "-" : ""
    return `${sign}${groupDigits(integer, group)}${fraction ? `${decimal}${fraction}` : ""}`
}

/** `percentage`, already between 0 and 100, with `decimals` places and a `%`. */
export const formatPercentage = (
    percentage: NumberInput,
    policy?: string | null,
    decimals: number = 2
): string => `${formatNumber(percentage, policy, decimals)}%`
