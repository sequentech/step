// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Timezones for every portal (VOTE-LIFECYCLE design §3–§4), with Intl only.
 *
 * Every rendered text goes through the `timezones.*` i18n keys so translation
 * overrides can change it; nothing here puts a zone text together itself. When
 * a key is missing, the defaults are the bundled ui-core translation of the
 * language and the language's own zone names from Intl (CLDR).
 */
import type {TFunction} from "i18next"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import catalanTranslation from "../translations/cat"
import frenchTranslation from "../translations/fr"
import tagalogTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"
import dutchTranslation from "../translations/nl"
import basqueTranslation from "../translations/eu"
import {timeZoneCountryCodes} from "./timeZoneCountries"
import {parseTranslationOverrideKey} from "./translationScopes"

export {timeZoneCountryCodes}

/** A zone as the picker lists it. */
export interface ITimeZoneOption {
    /** IANA name, tzdata canonical. */
    zone: string
    /** e.g. "(GMT+08:00) Manila" (timezones.option). */
    label: string
    /** e.g. "Philippines · Philippine Standard Time" (timezones.optionDetail). */
    detail: string
    /** Minutes east of UTC at the reference instant. */
    offsetMinutes: number
}

/** A wall time resolved in a zone. */
export interface IZonedInstant {
    /** The instant, as an ISO 8601 string in UTC. */
    instant: string
    kind: "exact" | "gap" | "overlap"
}

export interface ITimeZoneTextOptions {
    t: TFunction
    lang: string
    /**
     * The configured date format, applied to `{{dateTime}}`. It renders the
     * instant's wall time in `zone` and leaves the zone out (the texts add it).
     * Default: the language's medium date and short time.
     */
    formatDateTime?: (instant: Date, zone: string) => string
}

const MINUTE = 60_000
const DAY = 24 * 60 * MINUTE

// ---------------------------------------------------------------------------
// Zone names

/**
 * CLDR ids that Intl reports for zones tzdata has renamed. Links between
 * different places (Europe/Bratislava → Europe/Prague) are valid names of
 * their own and are not listed.
 */
const ALIASES: Record<string, string> = {
    "Africa/Asmera": "Africa/Asmara",
    "America/Buenos_Aires": "America/Argentina/Buenos_Aires",
    "America/Catamarca": "America/Argentina/Catamarca",
    "America/Coral_Harbour": "America/Atikokan",
    "America/Cordoba": "America/Argentina/Cordoba",
    "America/Godthab": "America/Nuuk",
    "America/Indianapolis": "America/Indiana/Indianapolis",
    "America/Jujuy": "America/Argentina/Jujuy",
    "America/Louisville": "America/Kentucky/Louisville",
    "America/Mendoza": "America/Argentina/Mendoza",
    "Asia/Calcutta": "Asia/Kolkata",
    "Asia/Dacca": "Asia/Dhaka",
    "Asia/Katmandu": "Asia/Kathmandu",
    "Asia/Rangoon": "Asia/Yangon",
    "Asia/Saigon": "Asia/Ho_Chi_Minh",
    "Asia/Ulan_Bator": "Asia/Ulaanbaatar",
    "Atlantic/Faeroe": "Atlantic/Faroe",
    "Europe/Kiev": "Europe/Kyiv",
    "Pacific/Enderbury": "Pacific/Kanton",
    "Pacific/Ponape": "Pacific/Pohnpei",
    "Pacific/Truk": "Pacific/Chuuk",
}

const supportedZones = new Map<string, boolean>()

/** Whether this engine's Intl knows the zone. */
const isSupportedZone = (zone: string): boolean => {
    let supported = supportedZones.get(zone)
    if (supported === undefined) {
        try {
            new Intl.DateTimeFormat("en", {timeZone: zone})
            supported = true
        } catch {
            supported = false
        }
        supportedZones.set(zone, supported)
    }
    return supported
}

/**
 * The tzdata canonical name of a zone (Asia/Calcutta → Asia/Kolkata). An
 * engine that doesn't know the new name keeps the old one.
 */
export const canonicalZone = (zone: string): string => {
    const trimmed = zone.trim()
    const target = Object.prototype.hasOwnProperty.call(ALIASES, trimmed)
        ? ALIASES[trimmed]
        : undefined
    return target !== undefined && isSupportedZone(target) ? target : trimmed
}

/** The old names of each canonical zone, so they stay searchable. */
const OLD_NAMES: Record<string, Array<string>> = Object.keys(ALIASES).reduce<
    Record<string, Array<string>>
>((names, alias) => {
    const canonical = ALIASES[alias]
    names[canonical] = [...(names[canonical] ?? []), alias]
    return names
}, {})

let zoneList: Array<string> | undefined

/** The browser's canonical zone, or UTC when the device zone is unsupported. */
export const browserTimeZone = (): string => {
    const zone = canonicalZone(new Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC")
    return isSupportedZone(zone) ? zone : "UTC"
}

/** Every IANA zone the browser knows, canonical, without duplicates. */
export const listTimeZones = (): Array<string> => {
    if (!zoneList) {
        const supported =
            typeof Intl.supportedValuesOf === "function"
                ? Intl.supportedValuesOf("timeZone")
                : Object.keys(timeZoneCountryCodes).filter(isSupportedZone)
        // UTC is what events without a configuration store, and the browser's
        // own zone may be one Intl doesn't list (Etc/GMT+5).
        const own = browserTimeZone()
        zoneList = Array.from(
            new Set([
                "UTC",
                ...(isSupportedZone(own) ? [own] : []),
                ...supported.map(canonicalZone),
            ])
        ).sort()
    }
    return zoneList.slice()
}

// ---------------------------------------------------------------------------
// Offsets and wall times

const formatters = new Map<string, Intl.DateTimeFormat>()

const formatter = (locale: string, format: Intl.DateTimeFormatOptions): Intl.DateTimeFormat => {
    const key = `${locale}|${JSON.stringify(format)}`
    let cached = formatters.get(key)
    if (!cached) {
        cached = new Intl.DateTimeFormat(locale, format)
        formatters.set(key, cached)
    }
    return cached
}

interface IWallTime {
    year: number
    month: number
    day: number
    hour: number
    minute: number
    second: number
    millisecond?: number
}

const wallTime = (at: Date, zone: string): IWallTime => {
    const parts = formatter("en-US", {
        timeZone: zone,
        hourCycle: "h23",
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
    }).formatToParts(at)
    const part = (type: Intl.DateTimeFormatPartTypes): number =>
        Number(parts.find((candidate) => candidate.type === type)?.value)
    return {
        year: part("year"),
        month: part("month"),
        day: part("day"),
        hour: part("hour"),
        minute: part("minute"),
        second: part("second"),
    }
}

/** A wall time read as if it were UTC, in milliseconds (years 0–99 included). */
const wallAsUtc = ({
    year,
    month,
    day,
    hour,
    minute,
    second,
    millisecond = 0,
}: IWallTime): number => {
    const date = new Date(Date.UTC(2000, 0, 1, hour, minute, second, millisecond))
    date.setUTCFullYear(year, month - 1, day)
    return date.getTime()
}

const floorToSecond = (milliseconds: number): number => Math.floor(milliseconds / 1000) * 1000

/** Milliseconds east of UTC of `zone` at `at`, to the second (old local mean times). */
const offsetMilliseconds = (zone: string, at: Date): number =>
    wallAsUtc(wallTime(at, zone)) - floorToSecond(at.getTime())

/** Minutes east of UTC of `zone` at `at`. */
export const zoneOffsetMinutes = (zone: string, at: Date): number =>
    Math.round(offsetMilliseconds(canonicalZone(zone), at) / MINUTE)

const LOCAL_PATTERN = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?$/

/** A wall time `YYYY-MM-DDTHH:MM[:SS[.mmm]]` read as if it were UTC, in milliseconds. */
const parseLocal = (local: string): number => {
    const match = LOCAL_PATTERN.exec(local.trim())
    if (!match) {
        throw new RangeError(`Invalid local date and time: ${local}`)
    }
    const [year, month, day, hour, minute] = match.slice(1, 6).map(Number)
    const second = match[6] === undefined ? 0 : Number(match[6])
    const millisecond = match[7] === undefined ? 0 : Number(match[7].padEnd(3, "0"))
    const value = wallAsUtc({year, month, day, hour, minute, second, millisecond})
    const date = new Date(value)
    if (
        date.getUTCFullYear() !== year ||
        date.getUTCMonth() !== month - 1 ||
        date.getUTCDate() !== day ||
        date.getUTCHours() !== hour ||
        date.getUTCMinutes() !== minute ||
        date.getUTCSeconds() !== second
    ) {
        throw new RangeError(`Invalid local date and time: ${local}`)
    }
    return value
}

const toIso = (milliseconds: number): string =>
    new Date(milliseconds).toISOString().replace(".000Z", "Z")

/**
 * The instant of a wall time (`YYYY-MM-DDTHH:MM`) in `zone`. A time that
 * happens twice (clocks go back) resolves to the first one (`overlap`); a time
 * that doesn't exist (clocks go forward) moves forward by the size of the gap,
 * as browsers do (`gap`). Throws a RangeError for a malformed wall time.
 */
export const zonedToInstant = (local: string, zone: string): IZonedInstant => {
    const canonical = canonicalZone(zone)
    const wall = parseLocal(local)
    const before = offsetMilliseconds(canonical, new Date(wall - DAY))
    const after = offsetMilliseconds(canonical, new Date(wall + DAY))
    const instants = Array.from(new Set([before, after]))
        .map((offset) => wall - offset)
        .filter(
            (instant) => wallAsUtc(wallTime(new Date(instant), canonical)) === floorToSecond(wall)
        )
        .sort((a, b) => a - b)
    if (instants.length === 0) {
        // The offset before the change puts the time after it, shifted forward.
        return {instant: toIso(wall - before), kind: "gap"}
    }
    return {instant: toIso(instants[0]), kind: instants.length > 1 ? "overlap" : "exact"}
}

const pad = (value: number): string => String(value).padStart(2, "0")

const toDate = (instant: Date | string): Date =>
    instant instanceof Date ? instant : new Date(instant)

/** The wall time (`YYYY-MM-DDTHH:MM`) of `instant` in `zone`. */
export const instantToZoned = (instant: Date | string, zone: string): string => {
    const {year, month, day, hour, minute} = wallTime(toDate(instant), canonicalZone(zone))
    return `${String(year).padStart(4, "0")}-${pad(month)}-${pad(day)}T${pad(hour)}:${pad(minute)}`
}

// ---------------------------------------------------------------------------
// Texts

type TimeZoneTexts = typeof englishTranslation.translations.timezones

// A copy taken at load: i18next keeps these objects as its resources, and
// overrides are written into them.
const snapshot = (texts: TimeZoneTexts): TimeZoneTexts => JSON.parse(JSON.stringify(texts))

const BUNDLED_TEXTS: Record<string, TimeZoneTexts> = {
    en: snapshot(englishTranslation.translations.timezones),
    es: snapshot(spanishTranslation.translations.timezones),
    cat: snapshot(catalanTranslation.translations.timezones),
    fr: snapshot(frenchTranslation.translations.timezones),
    tl: snapshot(tagalogTranslation.translations.timezones),
    gl: snapshot(galegoTranslation.translations.timezones),
    nl: snapshot(dutchTranslation.translations.timezones),
    eu: snapshot(basqueTranslation.translations.timezones),
}

const primaryLanguage = (lang: string): string => {
    const primary = lang.toLowerCase().split("-")[0]
    return primary === "ca" ? "cat" : primary
}

/** The ui-core default texts of a language, else English. */
const bundledTexts = (lang: string): TimeZoneTexts =>
    BUNDLED_TEXTS[lang] ?? BUNDLED_TEXTS[primaryLanguage(lang)] ?? BUNDLED_TEXTS.en

/** The Intl locale of a Step language code (Catalan is `cat` in Step). */
const intlLocale = (lang: string): string => {
    const locale = primaryLanguage(lang) === "cat" ? "ca" : lang
    try {
        return Intl.getCanonicalLocales(locale)[0] ?? "en"
    } catch {
        return "en"
    }
}

type TextValues = Record<string, string>

type Translate = (key: string, options: object) => unknown

const interpolate = (template: string, values: TextValues): string =>
    template.replace(/\{\{-?\s*(\w+)\s*(?:,[^}]*)?\}\}/g, (_, name: string) => values[name] ?? "")

/** `t` in the language, with a default and interpolation values. */
const text = (
    options: ITimeZoneTextOptions,
    key: string,
    defaultValue: string,
    values: TextValues = {}
): string => {
    const result = (options.t as unknown as Translate)(key, {
        ...values,
        lng: options.lang,
        defaultValue,
        interpolation: {escapeValue: false},
    })
    // A `t` without the ui-core translations returns the key.
    return typeof result === "string" && result !== key ? result : interpolate(defaultValue, values)
}

type ZoneTextKind = "abbr" | "name" | "city"

/**
 * The per-zone texts of a kind (`timezones.abbr`, …) in one lookup, so that
 * zones without an override cost no missing-key lookup each.
 */
const zoneTexts = (options: ITimeZoneTextOptions, kind: ZoneTextKind): Record<string, string> => {
    const result = (options.t as unknown as Translate)(`timezones.${kind}`, {
        lng: options.lang,
        returnObjects: true,
    })
    return result !== null && typeof result === "object" ? (result as Record<string, string>) : {}
}

/** One zone's text from the translations, else `fallback`. */
const zoneText = (texts: Record<string, string>, zone: string, fallback: string): string => {
    const value = Object.prototype.hasOwnProperty.call(texts, zone) ? texts[zone] : undefined
    return typeof value === "string" && value !== "" ? value : fallback
}

/** The language's own name of a zone (Intl/CLDR). */
const intlZoneName = (
    zone: string,
    lang: string,
    at: Date,
    style: "short" | "long" | "shortOffset"
): string => {
    try {
        return (
            formatter(intlLocale(lang), {timeZone: zone, timeZoneName: style})
                .formatToParts(at)
                .find((part) => part.type === "timeZoneName")?.value ?? zone
        )
    } catch {
        return zone
    }
}

/** What rendering zones in one language shares; each kind is read once, when needed. */
interface ITextContext {
    options: ITimeZoneTextOptions
    texts: (kind: ZoneTextKind) => Record<string, string>
}

const textContext = (options: ITimeZoneTextOptions): ITextContext => {
    const read: Partial<Record<ZoneTextKind, Record<string, string>>> = {}
    return {
        options,
        texts: (kind) => (read[kind] = read[kind] ?? zoneTexts(options, kind)),
    }
}

const labelOf = (context: ITextContext, zone: string, at: Date): string => {
    const bundled = bundledTexts(context.options.lang).abbr as Record<string, string>
    return zoneText(
        context.texts("abbr"),
        zone,
        zoneText(bundled, zone, intlZoneName(zone, context.options.lang, at, "short"))
    )
}

const nameOf = (context: ITextContext, zone: string, at: Date): string =>
    zoneText(context.texts("name"), zone, intlZoneName(zone, context.options.lang, at, "long"))

// POSIX-style Etc zones have the sign reversed: Etc/GMT+5 is UTC−05:00.
const ETC_OFFSET = /^Etc\/GMT([+-])(\d{1,2})$/

const cityOf = (context: ITextContext, zone: string): string => {
    const etc = ETC_OFFSET.exec(zone)
    const fallback = etc
        ? formatZoneOffset((etc[1] === "+" ? -60 : 60) * Number(etc[2]), context.options)
        : (zone.split("/").pop() ?? zone).replace(/_/g, " ")
    return zoneText(context.texts("city"), zone, fallback)
}

/** The short label of a zone at `at` (timezones.abbr.<zone>, else Intl's: PhST, EDT, GMT+4). */
export const zoneLabel = (
    zone: string,
    options: ITimeZoneTextOptions,
    at: Date = new Date()
): string => labelOf(textContext(options), canonicalZone(zone), at)

/** The long name of a zone (timezones.name.<zone>, else Intl's: Gulf Standard Time). */
export const zoneName = (
    zone: string,
    options: ITimeZoneTextOptions,
    at: Date = new Date()
): string => nameOf(textContext(options), canonicalZone(zone), at)

/** The city of a zone (timezones.city.<zone>, else the zone's last part: Buenos Aires). */
export const zoneCity = (zone: string, options: ITimeZoneTextOptions): string =>
    cityOf(textContext(options), canonicalZone(zone))

/** An offset as the picker shows it (timezones.offset: GMT+08:00). */
export const formatZoneOffset = (offsetMinutes: number, options: ITimeZoneTextOptions): string => {
    const minutes = Math.abs(offsetMinutes)
    return text(options, "timezones.offset", bundledTexts(options.lang).offset, {
        sign: offsetMinutes < 0 ? "-" : "+",
        hours: pad(Math.floor(minutes / 60)),
        minutes: pad(minutes % 60),
    })
}

const regionNames = new Map<string, Intl.DisplayNames | undefined>()

/** The countries of a zone, named in the language (Philippines). */
const countryNames = (zone: string, lang: string): Array<string> => {
    const codes = timeZoneCountryCodes[zone] ?? []
    const locale = intlLocale(lang)
    if (!regionNames.has(locale)) {
        try {
            regionNames.set(locale, new Intl.DisplayNames([locale], {type: "region"}))
        } catch {
            regionNames.set(locale, undefined)
        }
    }
    const names = regionNames.get(locale)
    return codes.map((code) => names?.of(code) ?? code)
}

const listFormats = new Map<string, Intl.ListFormat | undefined>()

const listOf = (items: Array<string>, lang: string): string => {
    const locale = intlLocale(lang)
    if (!listFormats.has(locale)) {
        try {
            listFormats.set(locale, new Intl.ListFormat(locale, {style: "short", type: "unit"}))
        } catch {
            listFormats.set(locale, undefined)
        }
    }
    return listFormats.get(locale)?.format(items) ?? items.join(", ")
}

const optionOf = (context: ITextContext, zone: string, at: Date): ITimeZoneOption => {
    const {options} = context
    const texts = bundledTexts(options.lang)
    const offsetMinutes = zoneOffsetMinutes(zone, at)
    const label = text(options, "timezones.option", texts.option, {
        offset: formatZoneOffset(offsetMinutes, options),
        city: cityOf(context, zone),
    })
    const name = nameOf(context, zone, at)
    const countries = countryNames(zone, options.lang)
    const detail = countries.length
        ? text(options, "timezones.optionDetail", texts.optionDetail, {
              countries: listOf(countries, options.lang),
              name,
          })
        : name
    return {zone, label, detail, offsetMinutes}
}

/** The picker option of one zone. */
export const timeZoneOption = (
    zone: string,
    options: ITimeZoneTextOptions,
    at: Date = new Date()
): ITimeZoneOption => optionOf(textContext(options), canonicalZone(zone), at)

/** The option of the event's primary zone (timezones.optionPrimary: … · primary). */
export const timeZonePrimaryOptionLabel = (
    option: ITimeZoneOption,
    options: ITimeZoneTextOptions
): string =>
    text(options, "timezones.optionPrimary", bundledTexts(options.lang).optionPrimary, {
        option: option.label,
    })

// ---------------------------------------------------------------------------
// Search

const normalize = (value: string): string =>
    value.normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[−–]/g, "-").toLowerCase().trim()

const SEPARATORS = /[\s/_,·()]+/
/** An offset word or query token: +05:45, -4, gmt+8, utc. */
const SIGNED = /^([+-]|gmt|utc)/

/** The words a query token may start. Offsets stay whole (+05:45). */
const words = (values: Array<string>): Array<string> => {
    const found = new Set<string>()
    for (const value of values) {
        for (const word of normalize(value).split(SEPARATORS)) {
            if (!word) continue
            found.add(word)
            // Hyphenated names (Port-au-Prince) also match by each part.
            if (!SIGNED.test(word)) {
                word.split("-").forEach((part) => part && found.add(part))
            }
        }
    }
    return Array.from(found)
}

/**
 * Whether a query token matches a word. An offset token matches a whole
 * offset (`+5` is +05:00, +05:30 and +05:45, never +10); others match the
 * start of a word.
 */
const tokenMatches = (token: string, word: string): boolean =>
    SIGNED.test(token) ? word === token || word.startsWith(`${token}:`) : word.startsWith(token)

/** Offset spellings people type: +8, GMT+8, UTC+08:00, +05:45, GMT+5:45. */
const offsetSpellings = (offsetMinutes: number): Array<string> => {
    const sign = offsetMinutes < 0 ? "-" : "+"
    const hours = Math.floor(Math.abs(offsetMinutes) / 60)
    const minutes = pad(Math.abs(offsetMinutes) % 60)
    const short = minutes === "00" ? `${sign}${hours}` : `${sign}${hours}:${minutes}`
    const long = `${sign}${pad(hours)}:${minutes}`
    return [short, long].flatMap((offset) => [offset, `gmt${offset}`, `utc${offset}`])
}

/** What a search compares for one zone. */
interface ISearchEntry {
    offsetMinutes: number
    city: string
    label: string
    ids: Array<string>
    words: Array<string>
    countryWords: Set<string>
}

const searchEntry = (context: ITextContext, zone: string, at: Date): ISearchEntry | null => {
    const {lang} = context.options
    let offsetMinutes: number
    try {
        offsetMinutes = zoneOffsetMinutes(zone, at)
    } catch {
        return null // A zone this browser doesn't know.
    }
    const city = cityOf(context, zone)
    const label = labelOf(context, zone, at)
    const countries = [...countryNames(zone, lang), ...countryNames(zone, "en")]
    const ids = [zone, ...(OLD_NAMES[zone] ?? [])]
    return {
        offsetMinutes,
        city,
        label,
        ids,
        countryWords: new Set(words(countries)),
        words: words([
            city,
            ...ids,
            label,
            nameOf(context, zone, at),
            ...countries,
            formatZoneOffset(offsetMinutes, context.options),
            intlZoneName(zone, lang, at, "shortOffset"),
            ...offsetSpellings(offsetMinutes),
        ]),
    }
}

// The entries of the last search (language, minute and texts), reused while
// the viewer types.
let searchCache: {key: string; entries: Map<string, ISearchEntry | null>} | undefined

const searchEntries = (
    context: ITextContext,
    at: Date
): {minute: Date; entries: Map<string, ISearchEntry | null>} => {
    const minute = new Date(Math.floor(at.getTime() / MINUTE) * MINUTE)
    const key = JSON.stringify([
        context.options.lang,
        minute.getTime(),
        context.texts("abbr"),
        context.texts("name"),
        context.texts("city"),
        formatZoneOffset(-345, context.options),
    ])
    if (searchCache?.key !== key) {
        searchCache = {key, entries: new Map()}
    }
    return {minute, entries: searchCache.entries}
}

/**
 * Zones matching a city, country, zone id (old names too), abbreviation, long
 * name or offset; every word of the query must match a word of the zone. The
 * best matches come first (the city itself, then the country, then a city
 * starting with the query, then the zone id or label), then by offset and
 * city. An empty query lists every zone.
 */
export const searchTimeZones = (
    query: string,
    zones: Array<string>,
    options: ITimeZoneTextOptions,
    at: Date = new Date()
): Array<ITimeZoneOption> => {
    const wanted = normalize(query)
    const tokens = wanted.split(SEPARATORS).filter(Boolean)
    const context = textContext(options)
    const {minute, entries} = searchEntries(context, at)
    const results: Array<{zone: string; entry: ISearchEntry; rank: number}> = []
    for (const zone of Array.from(new Set(zones.map(canonicalZone)))) {
        if (!entries.has(zone)) {
            entries.set(zone, searchEntry(context, zone, minute))
        }
        const entry = entries.get(zone)
        if (!entry) continue
        if (!tokens.every((token) => entry.words.some((word) => tokenMatches(token, word)))) {
            continue
        }
        const city = normalize(entry.city)
        const rank =
            city === wanted
                ? 0
                : tokens.length > 0 && tokens.every((token) => entry.countryWords.has(token))
                  ? 1
                  : city.startsWith(wanted)
                    ? 2
                    : [...entry.ids, entry.label].some((id) => normalize(id) === wanted)
                      ? 3
                      : 4
        results.push({zone, entry, rank})
    }
    return results
        .sort(
            (a, b) =>
                a.rank - b.rank ||
                a.entry.offsetMinutes - b.entry.offsetMinutes ||
                a.entry.city.localeCompare(b.entry.city)
        )
        .map(({zone}) => optionOf(context, zone, at))
}

// ---------------------------------------------------------------------------
// Times with a zone (combined strings)

/** The placeholders each combined string must keep (design §4). */
export const TIME_ZONE_COMBINED_TEXT_PLACEHOLDERS: Readonly<Record<string, ReadonlyArray<string>>> =
    {
        "timezones.dateTimeZone": ["dateTime", "zone"],
        "timezones.myTime": ["dateTime", "zone"],
        "timezones.placeTime": ["dateTime", "zone"],
        "timezones.voterDateTimeZone": ["dateTime", "zoneName"],
        "timezones.onThisDevice": ["dateTime"],
        "timezones.gap": ["dateTime", "city"],
        "timezones.overlap": ["dateTime", "city"],
    }

type CombinedText =
    | "dateTimeZone"
    | "myTime"
    | "placeTime"
    | "voterDateTimeZone"
    | "onThisDevice"
    | "gap"
    | "overlap"

const placeholderPattern = (name: string): RegExp =>
    new RegExp(`\\{\\{-?\\s*${name}\\s*(?:,[^}]*)?\\}\\}`)

/**
 * The placeholders a translation of a combined timezone string drops, for the
 * Localization tabs to refuse it. `key` may carry a scope (`global:`); keys
 * that aren't combined strings require nothing.
 */
export const missingTimeZonePlaceholders = (key: string, value: string): Array<string> => {
    const required = TIME_ZONE_COMBINED_TEXT_PLACEHOLDERS[parseTranslationOverrideKey(key).key]
    return (required ?? []).filter((name) => !placeholderPattern(name).test(value))
}

// Markers stand in for the values, so the rendered translation shows which
// placeholders it kept; the values replace them afterwards.
const marker = (name: string): string => `\u2063${name}\u2063`

const combined = (
    name: CombinedText,
    values: TextValues,
    options: ITimeZoneTextOptions
): string => {
    const key = `timezones.${name}`
    const fallback = bundledTexts(options.lang)[name]
    const markers = Object.fromEntries(Object.keys(values).map((value) => [value, marker(value)]))
    const rendered = text(options, key, fallback, markers)
    const missing = TIME_ZONE_COMBINED_TEXT_PLACEHOLDERS[key].filter(
        (placeholder) => !rendered.includes(marker(placeholder))
    )
    if (missing.length > 0) {
        console.warn(
            `The "${key}" translation for "${options.lang}" drops {{${missing.join(
                "}}, {{"
            )}}}; using the default.`
        )
        return interpolate(fallback, values)
    }
    return Object.entries(values).reduce(
        (result, [value, replacement]) => result.split(marker(value)).join(replacement),
        rendered
    )
}

const dateTimeText = (instant: Date, zone: string, options: ITimeZoneTextOptions): string =>
    options.formatDateTime
        ? options.formatDateTime(instant, zone)
        : formatter(intlLocale(options.lang), {
              timeZone: zone,
              dateStyle: "medium",
              timeStyle: "short",
          }).format(instant)

/** `{{dateTime}} {{zone}}` (timezones.dateTimeZone): admin times, reports. */
export const formatDateTimeZone = (
    instant: Date | string,
    zone: string,
    options: ITimeZoneTextOptions
): string => {
    const date = toDate(instant)
    const canonical = canonicalZone(zone)
    return combined(
        "dateTimeZone",
        {
            dateTime: dateTimeText(date, canonical, options),
            zone: zoneLabel(canonical, options, date),
        },
        options
    )
}

/** `{{dateTime}} {{zoneName}}` (timezones.voterDateTimeZone): voter screens. */
export const formatVoterDateTimeZone = (
    instant: Date | string,
    zone: string,
    options: ITimeZoneTextOptions
): string => {
    const date = toDate(instant)
    const canonical = canonicalZone(zone)
    return combined(
        "voterDateTimeZone",
        {
            dateTime: dateTimeText(date, canonical, options),
            zoneName: zoneName(canonical, options, date),
        },
        options
    )
}

/** `{{dateTime}} {{zone}} · my time` (timezones.myTime), in the viewer's zone. */
export const formatMyTime = (
    instant: Date | string,
    options: ITimeZoneTextOptions,
    myZone: string = browserTimeZone()
): string => {
    const date = toDate(instant)
    const canonical = canonicalZone(myZone)
    return combined(
        "myTime",
        {
            dateTime: dateTimeText(date, canonical, options),
            zone: zoneLabel(canonical, options, date),
        },
        options
    )
}

/** `{{dateTime}} {{zone}} · {{place}}` (timezones.placeTime): create and edit preview. */
export const formatPlaceTime = (
    instant: Date | string,
    zone: string,
    place: string,
    options: ITimeZoneTextOptions
): string => {
    const date = toDate(instant)
    const canonical = canonicalZone(zone)
    return combined(
        "placeTime",
        {
            dateTime: dateTimeText(date, canonical, options),
            zone: zoneLabel(canonical, options, date),
            place,
        },
        options
    )
}

/** `On this device: {{dateTime}}` (timezones.onThisDevice), in the device's zone. */
export const formatOnThisDevice = (
    instant: Date | string,
    options: ITimeZoneTextOptions,
    deviceZone: string = browserTimeZone()
): string =>
    combined(
        "onThisDevice",
        {dateTime: dateTimeText(toDate(instant), canonicalZone(deviceZone), options)},
        options
    )

/**
 * Why a wall time doesn't map to one instant in `zone` (timezones.gap or
 * timezones.overlap, with the wall time as typed), or undefined when it does.
 */
export const zonedTimeNote = (
    local: string,
    zone: string,
    options: ITimeZoneTextOptions
): string | undefined => {
    const {kind} = zonedToInstant(local, zone)
    if (kind === "exact") {
        return undefined
    }
    // The wall time as typed: the same digits, read in UTC.
    const typed = dateTimeText(new Date(parseLocal(local)), "UTC", options)
    return combined(kind, {dateTime: typed, city: zoneCity(zone, options)}, options)
}
