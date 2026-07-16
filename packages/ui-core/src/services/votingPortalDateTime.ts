// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EVotingPortalDateTimeFormat,
    IElectionEventPresentation,
    IVotingPortalCustomDateTimeFormat,
    VotingPortalDateTimeFormat,
} from "../types/ElectionEventPresentation"
import {translateFromPresentation} from "./translate"

/**
 * Localization key used to override the Voting Portal date/time format per
 * language, via `presentation.i18n[<lang>].votingPortalDateTimeFormat`.
 */
export const VOTING_PORTAL_DATETIME_FORMAT_KEY = "votingPortalDateTimeFormat"

/**
 * Minimal election event shape required to resolve the date/time format. Kept
 * loose so both voting-portal's `IElectionEvent` and ad-hoc objects can be passed
 * without coupling ui-core to a portal-specific type.
 */
export interface VotingPortalDateTimeEvent {
    id?: string | null
    presentation?: IElectionEventPresentation | null
}

export type DateTimeInput = Date | string | number

type Formatter = (date: Date) => string

/**
 * Thrown when an override pattern cannot be interpreted. Callers fall back to the
 * configured preset (never surfaced to voters).
 */
export class DateTimePatternError extends Error {
    constructor(message: string) {
        super(message)
        this.name = "DateTimePatternError"
    }
}

// Internal language codes that diverge from their BCP-47 tag. Only locale-sensitive
// presets need this; the override lookup always uses the raw internal code.
const INTERNAL_TO_BCP47: Record<string, string> = {cat: "ca"}
const toLocale = (lang: string): string => INTERNAL_TO_BCP47[lang] ?? lang

const pad = (value: number, length = 2): string => String(value).padStart(length, "0")

// Supported override tokens. Rendered in the voter's local time; any other
// characters in the pattern are passed through literally.
// Unicode LDML date field symbols (UTS #35 / CLDR)
const TOKEN_SOURCE = "yyyy|MM|dd|HH|mm|ss"
const hasToken = (pattern: string): boolean => new RegExp(TOKEN_SOURCE).test(pattern)

// Common tokens from other conventions (Moment-style YYYY/DD, 12-hour hh) that
// LDML assigns different meanings. Rendering them literally would silently
// corrupt voter-facing dates, so patterns containing them are rejected.
const MISUSED_TOKEN_SOURCE = "YYYY|DD|hh"

const tokenValue = (token: string, date: Date): string => {
    switch (token) {
        case "yyyy":
            return pad(date.getFullYear(), 4)
        case "MM":
            return pad(date.getMonth() + 1)
        case "dd":
            return pad(date.getDate())
        case "HH":
            return pad(date.getHours())
        case "mm":
            return pad(date.getMinutes())
        case "ss":
            return pad(date.getSeconds())
        default:
            return token
    }
}

/**
 * Narrows the stored policy to the inline custom-format variant
 * (`{custom: "<pattern>"}`). Presets are plain string enum values.
 */
export const isCustomVotingPortalDateTimeFormat = (
    value: VotingPortalDateTimeFormat | null | undefined
): value is IVotingPortalCustomDateTimeFormat =>
    typeof value === "object" && value !== null && typeof value.custom === "string"

/**
 * Validates and compiles an override pattern into a formatter. Throws a
 * {@link DateTimePatternError} on an empty pattern, one that contains no
 * recognized token, or one that contains a misused token from another
 * convention (YYYY, DD, hh). This is the single function that validates the
 * override, reused at admin save time and at render time.
 */
export const parseVotingPortalDateTimePattern = (pattern: string): Formatter => {
    if (!pattern || !pattern.trim()) {
        throw new DateTimePatternError("Empty date/time pattern")
    }
    const misused = pattern.match(new RegExp(MISUSED_TOKEN_SOURCE))
    if (misused) {
        throw new DateTimePatternError(
            `Unsupported token "${misused[0]}" in pattern: "${pattern}"; tokens are case-sensitive`
        )
    }
    if (!hasToken(pattern)) {
        throw new DateTimePatternError(`No recognized token in pattern: "${pattern}"`)
    }
    return (date: Date): string =>
        pattern.replace(new RegExp(TOKEN_SOURCE, "g"), (token) => tokenValue(token, date))
}

/**
 * Convenience predicate over {@link parseVotingPortalDateTimePattern} for callers
 * (admin inputs) that only need a valid/invalid answer rather than the formatter.
 */
export const isValidVotingPortalDateTimePattern = (pattern: string): boolean => {
    try {
        parseVotingPortalDateTimePattern(pattern)
        return true
    } catch {
        return false
    }
}

const legacyGb24h = (date: Date): string =>
    new Intl.DateTimeFormat("en-GB", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
    }).format(date)

// The CUSTOM policy is resolved from its inline pattern, not from this table.
type PresetDateTimeFormat = Exclude<EVotingPortalDateTimeFormat, EVotingPortalDateTimeFormat.CUSTOM>

const presetFormatters: Record<PresetDateTimeFormat, (date: Date, lang: string) => string> = {
    [EVotingPortalDateTimeFormat.LEGACY_GB_24H]: legacyGb24h,
    [EVotingPortalDateTimeFormat.ISO_LOCAL]: (date) =>
        `${pad(date.getFullYear(), 4)}-${pad(date.getMonth() + 1)}-${pad(
            date.getDate()
        )} ${pad(date.getHours())}:${pad(date.getMinutes())}`,
    [EVotingPortalDateTimeFormat.US_12H]: (date) =>
        new Intl.DateTimeFormat("en-US", {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "numeric",
            minute: "2-digit",
            hour12: true,
        }).format(date),
    [EVotingPortalDateTimeFormat.LOCALE_MEDIUM]: (date, lang) =>
        new Intl.DateTimeFormat(toLocale(lang), {
            dateStyle: "medium",
            timeStyle: "short",
        }).format(date),
    [EVotingPortalDateTimeFormat.DATE_ONLY]: (date, lang) =>
        new Intl.DateTimeFormat(toLocale(lang), {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
        }).format(date),
}

// Resolves the event-level policy (a preset or an inline custom pattern) to a
// formatter. An absent policy or an invalid custom pattern falls back to
// LEGACY_GB_24H; the custom pattern is validated by the same parser as the override.
const resolveConfiguredFormatter = (
    configured: VotingPortalDateTimeFormat | undefined,
    lang: string
): Formatter => {
    if (isCustomVotingPortalDateTimeFormat(configured)) {
        try {
            return parseVotingPortalDateTimePattern(configured.custom)
        } catch (error) {
            console.warn(
                `Invalid custom "${VOTING_PORTAL_DATETIME_FORMAT_KEY}" pattern "${configured.custom}"; falling back to the legacy format.`,
                error
            )
            return (date: Date) =>
                presetFormatters[EVotingPortalDateTimeFormat.LEGACY_GB_24H](date, lang)
        }
    }
    const preset =
        (configured && presetFormatters[configured as PresetDateTimeFormat]) ??
        presetFormatters[EVotingPortalDateTimeFormat.LEGACY_GB_24H]
    return (date: Date) => preset(date, lang)
}

const resolvePreset = (event: VotingPortalDateTimeEvent | null | undefined): Formatter =>
    resolveConfiguredFormatter(event?.presentation?.voting_portal_datetime_format, "en")

// Memoizes the resolved formatter per (eventId, lang) so resolution is O(1) per
// render and issues no extra work. Resolution is stable for a loaded event.
const formatterCache = new Map<string, Formatter>()

const buildFormatter = (
    event: VotingPortalDateTimeEvent | null | undefined,
    lang: string
): Formatter => {
    const override = translateFromPresentation(event, VOTING_PORTAL_DATETIME_FORMAT_KEY, lang)
    if (override) {
        try {
            return parseVotingPortalDateTimePattern(override)
        } catch (error) {
            console.warn(
                `Invalid "${VOTING_PORTAL_DATETIME_FORMAT_KEY}" override "${override}" for language "${lang}"; falling back to the configured preset.`,
                error
            )
        }
    }
    return resolveConfiguredFormatter(event?.presentation?.voting_portal_datetime_format, lang)
}

const toDate = (input: DateTimeInput): Date => (input instanceof Date ? input : new Date(input))

/**
 * Resolves and formats a date/time for voter-facing surfaces of the Voting Portal.
 *
 * Resolution order: per-language translation override → event preset →
 * `LEGACY_GB_24H`. A malformed override logs a warning and falls back to the
 * preset; formatting never throws to the voter.
 *
 * @param date the instant to format (Date, ISO string, or epoch milliseconds)
 * @param event the election event (carrying `presentation`)
 * @param lang the active voter language (internal code, e.g. `en`, `cat`)
 */
export const formatVotingPortalDateTime = (
    date: DateTimeInput,
    event: VotingPortalDateTimeEvent | null | undefined,
    lang: string
): string => {
    const parsedDate = toDate(date)
    const cacheKey = `${event?.id ?? "unknown"}:${lang}`
    let formatter = formatterCache.get(cacheKey)
    if (!formatter) {
        formatter = buildFormatter(event, lang)
        formatterCache.set(cacheKey, formatter)
    }
    try {
        return formatter(parsedDate)
    } catch (error) {
        console.warn("Voting Portal date/time formatting failed; using legacy format.", error)
        return resolvePreset(null)(parsedDate)
    }
}

/** Test-only: clears the per-(eventId, lang) formatter memo. */
export const clearVotingPortalDateTimeCache = (): void => {
    formatterCache.clear()
}
