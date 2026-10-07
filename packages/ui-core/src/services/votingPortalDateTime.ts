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
import {ETranslationScope, filterTranslationOverrides} from "./translationScopes"

/**
 * Localization key used to override the Voting Portal date/time format per
 * language, via a Voting Portal, global, or legacy translation override.
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

/**
 * Formats an instant. `timeZone` is an IANA zone; without one the browser's
 * zone is used. The zone only moves the wall clock: it is never written into
 * the text (callers name it through `timezones.voterDateTimeZone`).
 */
type Formatter = (date: Date, timeZone?: string) => string

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

// Supported override tokens. Rendered in the requested zone (the browser's when
// none is given); any other characters in the pattern are passed through literally.
// Unicode LDML date field symbols (UTS #35 / CLDR)
const TOKEN_SOURCE = "yyyy|MM|dd|HH|mm|ss"
const hasToken = (pattern: string): boolean => new RegExp(TOKEN_SOURCE).test(pattern)

// Common tokens from other conventions (Moment-style YYYY/DD, 12-hour hh) that
// LDML assigns different meanings. Rendering them literally would silently
// corrupt voter-facing dates, so patterns containing them are rejected.
const MISUSED_TOKEN_SOURCE = "YYYY|DD|hh"

/** The wall-clock fields of an instant in a zone. */
interface WallClock {
    year: number
    month: number
    day: number
    hour: number
    minute: number
    second: number
}

// One Intl formatter per zone; building them is the expensive part.
const wallClockFormatters = new Map<string, Intl.DateTimeFormat>()

const wallClockFormatter = (timeZone: string | undefined): Intl.DateTimeFormat => {
    const key = timeZone ?? ""
    let formatter = wallClockFormatters.get(key)
    if (!formatter) {
        // Throws a RangeError on an unknown zone, which the caller turns into
        // the logged fallback.
        formatter = new Intl.DateTimeFormat("en-US", {
            timeZone,
            year: "numeric",
            month: "numeric",
            day: "numeric",
            hour: "numeric",
            minute: "numeric",
            second: "numeric",
            hourCycle: "h23",
        })
        wallClockFormatters.set(key, formatter)
    }
    return formatter
}

const wallClock = (date: Date, timeZone?: string): WallClock => {
    const fields: Record<string, number> = {}
    for (const part of wallClockFormatter(timeZone).formatToParts(date)) {
        if (part.type !== "literal") {
            fields[part.type] = Number(part.value)
        }
    }
    return {
        year: fields.year,
        month: fields.month,
        day: fields.day,
        // Some engines still print midnight as 24 with h23.
        hour: fields.hour % 24,
        minute: fields.minute,
        second: fields.second,
    }
}

const tokenValue = (token: string, clock: WallClock): string => {
    switch (token) {
        case "yyyy":
            return pad(clock.year, 4)
        case "MM":
            return pad(clock.month)
        case "dd":
            return pad(clock.day)
        case "HH":
            return pad(clock.hour)
        case "mm":
            return pad(clock.minute)
        case "ss":
            return pad(clock.second)
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
    return (date: Date, timeZone?: string): string => {
        const clock = wallClock(date, timeZone)
        return pattern.replace(new RegExp(TOKEN_SOURCE, "g"), (token) => tokenValue(token, clock))
    }
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

const legacyGb24h = (date: Date, timeZone?: string): string =>
    new Intl.DateTimeFormat("en-GB", {
        timeZone,
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
    }).format(date)

// The CUSTOM policy is resolved from its inline pattern, not from this table.
type PresetDateTimeFormat = Exclude<EVotingPortalDateTimeFormat, EVotingPortalDateTimeFormat.CUSTOM>

const presetFormatters: Record<
    PresetDateTimeFormat,
    (date: Date, lang: string, timeZone?: string) => string
> = {
    [EVotingPortalDateTimeFormat.LEGACY_GB_24H]: (date, _lang, timeZone) =>
        legacyGb24h(date, timeZone),
    [EVotingPortalDateTimeFormat.ISO_LOCAL]: (date, _lang, timeZone) => {
        const clock = wallClock(date, timeZone)
        return `${pad(clock.year, 4)}-${pad(clock.month)}-${pad(clock.day)} ${pad(
            clock.hour
        )}:${pad(clock.minute)}`
    },
    [EVotingPortalDateTimeFormat.US_12H]: (date, _lang, timeZone) =>
        new Intl.DateTimeFormat("en-US", {
            timeZone,
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "numeric",
            minute: "2-digit",
            hour12: true,
        }).format(date),
    [EVotingPortalDateTimeFormat.LOCALE_MEDIUM]: (date, lang, timeZone) =>
        new Intl.DateTimeFormat(toLocale(lang), {
            timeZone,
            dateStyle: "medium",
            timeStyle: "short",
        }).format(date),
    [EVotingPortalDateTimeFormat.DATE_ONLY]: (date, lang, timeZone) =>
        new Intl.DateTimeFormat(toLocale(lang), {
            timeZone,
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
            return (date: Date, timeZone?: string) =>
                presetFormatters[EVotingPortalDateTimeFormat.LEGACY_GB_24H](date, lang, timeZone)
        }
    }
    const preset =
        (configured && presetFormatters[configured as PresetDateTimeFormat]) ??
        presetFormatters[EVotingPortalDateTimeFormat.LEGACY_GB_24H]
    return (date: Date, timeZone?: string) => preset(date, lang, timeZone)
}

const resolvePreset = (event: VotingPortalDateTimeEvent | null | undefined): Formatter =>
    resolveConfiguredFormatter(event?.presentation?.voting_portal_datetime_format, "en")

// Memoizes the resolved formatter per (eventId, lang, zone) so resolution is O(1)
// per render and issues no extra work. Resolution is stable for a loaded event.
const formatterCache = new Map<string, Formatter>()

const buildFormatter = (
    event: VotingPortalDateTimeEvent | null | undefined,
    lang: string
): Formatter => {
    const scopedI18n = filterTranslationOverrides(
        event?.presentation?.i18n,
        ETranslationScope.VOTING_PORTAL,
        ETranslationScope.VOTING_PORTAL
    )
    const override = translateFromPresentation(
        {i18n: scopedI18n},
        VOTING_PORTAL_DATETIME_FORMAT_KEY,
        lang
    )
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
 * The text holds the date and time only. To name the zone, pass this as the
 * `{{dateTime}}` of `timezones.voterDateTimeZone` (see `formatVoterDateTimeZone`).
 *
 * @param date the instant to format (Date, ISO string, or epoch milliseconds)
 * @param event the election event (carrying `presentation`)
 * @param lang the active voter language (internal code, e.g. `en`, `cat`)
 * @param timeZone the IANA zone whose wall clock is shown; the browser's when absent.
 *        An unknown zone logs a warning and falls back to the legacy format in
 *        the browser's zone.
 */
export const formatVotingPortalDateTime = (
    date: DateTimeInput,
    event: VotingPortalDateTimeEvent | null | undefined,
    lang: string,
    timeZone?: string
): string => {
    const parsedDate = toDate(date)
    if (Number.isNaN(parsedDate.getTime())) {
        // Not an instant: nothing to show, and nothing a fallback could format.
        return "-"
    }
    const cacheKey = `${event?.id ?? "unknown"}:${lang}:${timeZone ?? ""}`
    let formatter = formatterCache.get(cacheKey)
    if (!formatter) {
        formatter = buildFormatter(event, lang)
        formatterCache.set(cacheKey, formatter)
    }
    try {
        return formatter(parsedDate, timeZone)
    } catch (error) {
        console.warn(
            `Voting Portal date/time formatting failed${
                timeZone ? ` in timezone "${timeZone}"` : ""
            }; using legacy format.`,
            error
        )
        return resolvePreset(null)(parsedDate)
    }
}

/** Test-only: clears the per-(eventId, lang, zone) formatter memo. */
export const clearVotingPortalDateTimeCache = (): void => {
    formatterCache.clear()
}
