// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The VOTE-LIFECYCLE proof checks (search, labels, DST gaps and overlaps,
// fractional offsets, defaults and overrides), run against Node's Intl.
// "My time" and the device zone are passed explicitly (New York), so the
// tests pass in any machine zone.
import {afterEach, beforeAll, describe, expect, it, jest} from "@jest/globals"
import type {TFunction} from "i18next"
import i18n, {initializeLanguages, overwriteTranslations} from "./i18n"
import {ETranslationScope} from "./translationScopes"
import {
    browserTimeZone,
    canonicalZone,
    formatDateTimeZone,
    formatMyTime,
    formatOnThisDevice,
    formatPlaceTime,
    formatVoterDateTimeZone,
    formatZoneOffset,
    instantToZoned,
    ITimeZoneTextOptions,
    listTimeZones,
    missingTimeZonePlaceholders,
    searchTimeZones,
    TIME_ZONE_COMBINED_TEXT_PLACEHOLDERS,
    timeZoneCountryCodes,
    timeZoneOption,
    timeZonePrimaryOptionLabel,
    zoneCity,
    zonedTimeNote,
    zonedToInstant,
    zoneLabel,
    zoneName,
    zoneOffsetMinutes,
} from "./timeZones"
import englishTranslation from "../translations/en"
import spanishTranslation from "../translations/es"
import catalanTranslation from "../translations/cat"
import frenchTranslation from "../translations/fr"
import tagalogTranslation from "../translations/tl"
import galegoTranslation from "../translations/gl"
import dutchTranslation from "../translations/nl"
import basqueTranslation from "../translations/eu"

jest.mock("sequent-core", () => ({
    iso_639_2t_to_bcp47_js: (language: string) => language,
    locale_to_internal_language_code_js: (language: string) => language,
}))

const BUNDLES = {
    en: englishTranslation,
    es: spanishTranslation,
    cat: catalanTranslation,
    fr: frenchTranslation,
    tl: tagalogTranslation,
    gl: galegoTranslation,
    nl: dutchTranslation,
    eu: basqueTranslation,
}

/** The viewer of the ticket's examples. */
const NEW_YORK = "America/New_York"

/** The sample schedule: voting opens at 00:00 Post time on 9 April 2028. */
const OPENING = "2028-04-09T00:00"
/** 19:00 Manila on 8 May 2028, the sample close of polls. */
const CLOSE = new Date("2028-05-08T11:00:00Z")

// en-GB "09 Apr 2028, 00:00", the Admin Portal style of the ticket's examples.
const adminFormat = (instant: Date, zone: string): string =>
    new Intl.DateTimeFormat("en-GB", {
        timeZone: zone,
        day: "2-digit",
        month: "short",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit",
        hourCycle: "h23",
    }).format(instant)

const options = (lang = "en", formatDateTime?: ITimeZoneTextOptions["formatDateTime"]) => ({
    t: i18n.t.bind(i18n) as TFunction,
    lang,
    formatDateTime,
})

/** ICU writes a narrow no-break space before AM/PM. */
const spaces = (text: string): string => text.replace(/[\u202f\u00a0]/g, " ")

const intlName = (zone: string, lang: string, at: Date, style: "short" | "long") =>
    new Intl.DateTimeFormat(lang, {timeZone: zone, timeZoneName: style})
        .formatToParts(at)
        .find((part) => part.type === "timeZoneName")?.value

const ADMIN_SCOPE = {scope: ETranslationScope.ADMIN_PORTAL, changeDefaultLanguage: false} as const

/** Saves tenant overrides as Settings > Localization does (scope:key). */
const override = (lang: string, texts: Record<string, string>) =>
    overwriteTranslations(
        {
            i18n: {
                [lang]: Object.fromEntries(
                    Object.entries(texts).map(([key, value]) => [`adminPortal:${key}`, value])
                ),
            },
        },
        ADMIN_SCOPE
    )

beforeAll(() => {
    initializeLanguages({}, "en")
})

afterEach(() => {
    overwriteTranslations({i18n: {}}, ADMIN_SCOPE)
    jest.restoreAllMocks()
})

describe("zone list and names", () => {
    it("stores tzdata names for the CLDR names browsers report", () => {
        expect(canonicalZone("Asia/Calcutta")).toBe("Asia/Kolkata")
        expect(canonicalZone("Asia/Katmandu")).toBe("Asia/Kathmandu")
        expect(canonicalZone("Asia/Rangoon")).toBe("Asia/Yangon")
        expect(canonicalZone("Europe/Kiev")).toBe("Europe/Kyiv")
        expect(canonicalZone("America/Buenos_Aires")).toBe("America/Argentina/Buenos_Aires")
        expect(canonicalZone(" Asia/Saigon ")).toBe("Asia/Ho_Chi_Minh")
        // Canonical names and valid links stay as they are.
        expect(canonicalZone("Europe/Madrid")).toBe("Europe/Madrid")
        expect(canonicalZone("Europe/Bratislava")).toBe("Europe/Bratislava")
    })

    it("lists every zone once, canonical, with UTC", () => {
        const zones = listTimeZones()
        expect(zones).toContain("Asia/Kolkata")
        expect(zones).toContain("Asia/Manila")
        expect(zones).toContain("UTC")
        expect(zones).not.toContain("Asia/Calcutta")
        expect(new Set(zones).size).toBe(zones.length)
        expect(zones.every((zone) => canonicalZone(zone) === zone)).toBe(true)
        expect(zones.length).toBeGreaterThan(400)
    })

    it("reports the browser's zone canonical", () => {
        expect(browserTimeZone()).toBe(
            canonicalZone(new Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC")
        )
        expect(listTimeZones()).toContain(browserTimeZone())
    })

    it("keeps the countries of zones shared by several countries right", () => {
        expect(timeZoneCountryCodes["Asia/Manila"]).toEqual(["PH"])
        expect(timeZoneCountryCodes["Asia/Dubai"]).toEqual(["AE"])
        expect(timeZoneCountryCodes["Europe/Paris"]).toEqual(["FR"])
        expect(timeZoneCountryCodes["Atlantic/Canary"]).toEqual(["ES"])
    })
})

describe("timezone picker", () => {
    it("finds Manila by city: (GMT+08:00) Manila, Philippines and its long name", () => {
        const [first] = searchTimeZones("Manila", listTimeZones(), options())
        expect(first).toEqual({
            zone: "Asia/Manila",
            label: "(GMT+08:00) Manila",
            detail: "Philippines · Philippine Standard Time",
            offsetMinutes: 480,
        })
    })

    it("finds zones by country, zone id, abbreviation, long name and offset", () => {
        const zonesOf = (query: string) =>
            searchTimeZones(query, listTimeZones(), options()).map((option) => option.zone)
        expect(zonesOf("Philippines")).toContain("Asia/Manila")
        expect(zonesOf("philippines")[0]).toBe("Asia/Manila")
        expect(zonesOf("Asia/Dubai")[0]).toBe("Asia/Dubai")
        expect(zonesOf("PhST")).toEqual(["Asia/Manila"])
        expect(zonesOf("gulf standard")).toContain("Asia/Dubai")
        expect(zonesOf("+05:45")).toContain("Asia/Kathmandu")
        expect(zonesOf("GMT+5:45")).toContain("Asia/Kathmandu")
        expect(zonesOf("UTC+05:30")).toContain("Asia/Kolkata")
        expect(zonesOf("new york")[0]).toBe("America/New_York")
        expect(zonesOf("buenos")).toContain("America/Argentina/Buenos_Aires")
        expect(zonesOf("zzzz")).toEqual([])
    })

    it("matches whole offsets, never the start of a longer one", () => {
        const offsetsOf = (query: string) =>
            searchTimeZones(query, listTimeZones(), options(), CLOSE).map(
                (option) => option.offsetMinutes
            )
        const gmtPlusOne = offsetsOf("GMT+1")
        expect(gmtPlusOne.length).toBeGreaterThan(0)
        expect(gmtPlusOne.every((minutes) => minutes >= 60 && minutes < 120)).toBe(true)
        expect(new Set(offsetsOf("+5"))).toEqual(new Set([300, 330, 345]))
        expect(offsetsOf("+0").every((minutes) => minutes === 0)).toBe(true)
        expect(offsetsOf("-4").every((minutes) => minutes === -240)).toBe(true)
        expect(offsetsOf("gmt").every((minutes) => minutes === 0)).toBe(true)
        const utc = searchTimeZones("UTC", listTimeZones(), options(), CLOSE)
        expect(utc[0].zone).toBe("UTC")
        expect(utc.length).toBeLessThan(10)
    })

    it("ranks a country's zones above other matches, and finds old names", () => {
        const zonesOf = (query: string) =>
            searchTimeZones(query, listTimeZones(), options(), CLOSE).map((option) => option.zone)
        expect(zonesOf("India")[0]).toBe("Asia/Kolkata")
        const spain = zonesOf("Spain")
        expect(spain.indexOf("Europe/Madrid")).toBeGreaterThanOrEqual(0)
        expect(spain.indexOf("Europe/Madrid")).toBeLessThan(spain.indexOf("America/Port_of_Spain"))
        expect(zonesOf("Calcutta")).toEqual(["Asia/Kolkata"])
        expect(zonesOf("Asia/Calcutta")).toEqual(["Asia/Kolkata"])
        expect(zonesOf("Kiev")).toContain("Europe/Kyiv")
        expect(zonesOf("Saigon")).toContain("Asia/Ho_Chi_Minh")
    })

    it("names Etc zones by their real offset (Etc/GMT+5 is GMT-05:00)", () => {
        expect(zoneCity("Etc/GMT+5", options())).toBe("GMT-05:00")
        expect(zoneCity("Etc/GMT-14", options())).toBe("GMT+14:00")
        expect(timeZoneOption("Etc/GMT+5", options(), CLOSE)).toMatchObject({
            label: "(GMT-05:00) GMT-05:00",
            offsetMinutes: -300,
        })
    })

    it("searches in the viewer's language", () => {
        const zones = searchTimeZones("Filipinas", listTimeZones(), options("es"))
        expect(zones[0].zone).toBe("Asia/Manila")
        expect(zones[0].detail).toBe("Filipinas · hora estándar de Filipinas")
    })

    it("lists aliases once, under the tzdata name, sorted by offset", () => {
        const found = searchTimeZones(
            "",
            ["Asia/Calcutta", "Asia/Kolkata", "Asia/Manila"],
            options()
        )
        expect(found.map((option) => option.zone)).toEqual(["Asia/Kolkata", "Asia/Manila"])
        expect(found[0].label).toBe("(GMT+05:30) Kolkata")
    })

    it("marks the primary option", () => {
        const option = timeZoneOption("Asia/Manila", options(), CLOSE)
        expect(timeZonePrimaryOptionLabel(option, options())).toBe("(GMT+08:00) Manila · primary")
        expect(timeZonePrimaryOptionLabel(option, options("es"))).toBe(
            "(GMT+08:00) Manila · principal"
        )
    })

    it("shows negative and UTC offsets", () => {
        expect(timeZoneOption("America/Toronto", options(), CLOSE).label).toBe(
            "(GMT-04:00) Toronto"
        )
        expect(timeZoneOption("UTC", options(), CLOSE)).toEqual({
            zone: "UTC",
            label: "(GMT+00:00) UTC",
            detail: "Coordinated Universal Time",
            offsetMinutes: 0,
        })
        expect(formatZoneOffset(-570, options())).toBe("GMT-09:30")
    })
})

describe("wall time and instant", () => {
    it("resolves the ticket's example: 00:00 in Dubai is 20:00 UTC the day before", () => {
        expect(zonedToInstant(OPENING, "Asia/Dubai")).toEqual({
            instant: "2028-04-08T20:00:00Z",
            kind: "exact",
        })
        expect(instantToZoned("2028-04-08T20:00:00Z", "Asia/Dubai")).toBe(OPENING)
    })

    it.each([
        ["Asia/Kathmandu", 345, "2028-04-08T18:15:00Z"],
        ["Asia/Kolkata", 330, "2028-04-08T18:30:00Z"],
        ["Asia/Calcutta", 330, "2028-04-08T18:30:00Z"],
        ["Asia/Tehran", 210, "2028-04-08T20:30:00Z"],
        ["Asia/Yangon", 390, "2028-04-08T17:30:00Z"],
        ["Asia/Manila", 480, "2028-04-08T16:00:00Z"],
    ])("opens %s (%i minutes east) at local midnight", (zone, minutes, instant) => {
        expect(zoneOffsetMinutes(zone, new Date(instant))).toBe(minutes)
        expect(zonedToInstant(OPENING, zone)).toEqual({instant, kind: "exact"})
        expect(instantToZoned(instant, zone)).toBe(OPENING)
    })

    it("shifts a time in the Toronto DST gap forward, as browsers do", () => {
        const resolved = zonedToInstant("2028-03-12T02:30", "America/Toronto")
        expect(resolved).toEqual({instant: "2028-03-12T07:30:00Z", kind: "gap"})
        expect(instantToZoned(resolved.instant, "America/Toronto")).toBe("2028-03-12T03:30")
    })

    it("takes the first of the two Toronto times when clocks go back", () => {
        const resolved = zonedToInstant("2028-11-05T01:30", "America/Toronto")
        expect(resolved).toEqual({instant: "2028-11-05T05:30:00Z", kind: "overlap"})
        expect(zoneLabel("America/Toronto", options(), new Date(resolved.instant))).toBe("EDT")
    })

    it("follows Cairo's DST change on 28 April 2028", () => {
        expect(zonedToInstant(OPENING, "Africa/Cairo")).toEqual({
            instant: "2028-04-08T22:00:00Z",
            kind: "exact",
        })
        expect(zoneOffsetMinutes("Africa/Cairo", new Date("2028-04-27T12:00:00Z"))).toBe(120)
        expect(zoneOffsetMinutes("Africa/Cairo", new Date("2028-04-28T12:00:00Z"))).toBe(180)
        expect(zonedToInstant("2028-04-28T00:30", "Africa/Cairo")).toEqual({
            instant: "2028-04-27T22:30:00Z",
            kind: "gap",
        })
        expect(zonedToInstant("2028-04-28T12:00", "Africa/Cairo")).toEqual({
            instant: "2028-04-28T09:00:00Z",
            kind: "exact",
        })
    })

    it("accepts seconds and refuses what is not a wall time", () => {
        expect(zonedToInstant("2028-04-09T00:00:30", "UTC").instant).toBe("2028-04-09T00:00:30Z")
        expect(() => zonedToInstant("2028-04-09 00:00Z", "UTC")).toThrow(RangeError)
        expect(() => zonedToInstant("2028-02-30T00:00", "UTC")).toThrow(RangeError)
    })

    it("accepts milliseconds from step inputs", () => {
        expect(zonedToInstant("2028-04-09T00:00:30.5", "UTC").instant).toBe(
            "2028-04-09T00:00:30.500Z"
        )
        expect(zonedToInstant("2028-04-09T00:00:00.000", "Asia/Dubai")).toEqual({
            instant: "2028-04-08T20:00:00Z",
            kind: "exact",
        })
    })

    it("keeps years before 100 (no two-digit year rule)", () => {
        expect(zonedToInstant("0020-03-08T10:00", "UTC")).toEqual({
            instant: "0020-03-08T10:00:00Z",
            kind: "exact",
        })
        // New York kept local mean time then (-04:56:02).
        const newYork = zonedToInstant("0020-03-08T10:00", NEW_YORK)
        expect(newYork).toEqual({instant: "0020-03-08T14:56:02Z", kind: "exact"})
        expect(instantToZoned(newYork.instant, NEW_YORK)).toBe("0020-03-08T10:00")
        expect(() => zonedToInstant("0020-02-30T10:00", "UTC")).toThrow(RangeError)
    })
})

describe("engines that differ", () => {
    const isolated = (): typeof import("./timeZones") => {
        let module: typeof import("./timeZones") | undefined
        jest.isolateModules(() => {
            module = require("./timeZones")
        })
        return module as typeof import("./timeZones")
    }

    it("keeps an old name when the engine doesn't know the new one", () => {
        const DateTimeFormat = Intl.DateTimeFormat
        jest.spyOn(Intl, "DateTimeFormat").mockImplementation(((
            locale?: string | Array<string>,
            format?: Intl.DateTimeFormatOptions
        ) => {
            if (format?.timeZone === "Asia/Kolkata") throw new RangeError("Invalid time zone")
            return new DateTimeFormat(locale, format)
        }) as unknown as typeof Intl.DateTimeFormat)
        const service = isolated()
        expect(service.canonicalZone("Asia/Calcutta")).toBe("Asia/Calcutta")
        expect(service.canonicalZone("Europe/Kiev")).toBe("Europe/Kyiv")
        expect(service.listTimeZones()).toContain("Asia/Calcutta")
        expect(service.zonedToInstant(OPENING, "Asia/Calcutta").instant).toBe(
            "2028-04-08T18:30:00Z"
        )
    })

    it("uses UTC when the device reports an unsupported timezone", () => {
        const resolvedOptions = Intl.DateTimeFormat.prototype.resolvedOptions
        jest.spyOn(Intl.DateTimeFormat.prototype, "resolvedOptions").mockImplementation(function (
            this: Intl.DateTimeFormat
        ) {
            return {...resolvedOptions.call(this), timeZone: "Etc/Unknown"}
        })
        const service = isolated()
        expect(service.browserTimeZone()).toBe("UTC")
        expect(service.formatMyTime("2028-04-08T20:00:00Z", options())).toContain("UTC")
        expect(service.listTimeZones()).not.toContain("Etc/Unknown")
    })

    it("lists the browser's zone even when Intl doesn't (Etc/GMT+5)", () => {
        const resolvedOptions = Intl.DateTimeFormat.prototype.resolvedOptions
        jest.spyOn(Intl.DateTimeFormat.prototype, "resolvedOptions").mockImplementation(function (
            this: Intl.DateTimeFormat
        ) {
            return {...resolvedOptions.call(this), timeZone: "Etc/GMT+5"}
        })
        const service = isolated()
        expect(service.browserTimeZone()).toBe("Etc/GMT+5")
        expect(service.listTimeZones()).toContain("Etc/GMT+5")
    })
})

describe("labels and names", () => {
    it("labels Manila PhST and other zones with the language's short name", () => {
        expect(zoneLabel("Asia/Manila", options(), CLOSE)).toBe("PhST")
        expect(zoneLabel("Asia/Dubai", options(), CLOSE)).toBe("GMT+4")
        expect(zoneLabel("America/New_York", options(), CLOSE)).toBe("EDT")
        expect(zoneLabel("America/New_York", options(), new Date("2028-01-10T12:00:00Z"))).toBe(
            "EST"
        )
        expect(zoneLabel("Asia/Calcutta", options(), CLOSE)).toBe("GMT+5:30")
    })

    it("names zones in words in the voter's language", () => {
        expect(zoneName("Asia/Dubai", options(), CLOSE)).toBe("Gulf Standard Time")
        expect(zoneName("Asia/Manila", options("es"), CLOSE)).toBe("hora estándar de Filipinas")
        // Catalan is "cat" inside Step and "ca" for Intl.
        expect(zoneName("Europe/Madrid", options("cat"), CLOSE)).toBe(
            intlName("Europe/Madrid", "ca", CLOSE, "long")
        )
    })

    it("uses the last part of the zone as its city", () => {
        expect(zoneCity("America/Argentina/Buenos_Aires", options())).toBe("Buenos Aires")
        expect(zoneCity("UTC", options())).toBe("UTC")
    })
})

describe("times with a zone (defaults)", () => {
    const opening = new Date("2028-04-08T20:00:00Z")

    it("writes admin times as {{dateTime}} {{zone}} and my time", () => {
        const admin = options("en", adminFormat)
        expect(formatDateTimeZone(opening, "Asia/Dubai", admin)).toBe("09 Apr 2028, 00:00 GMT+4")
        expect(formatMyTime(opening, admin, NEW_YORK)).toBe("08 Apr 2028, 16:00 EDT · my time")
        expect(formatPlaceTime(opening, "Asia/Dubai", "Dubai PCG", admin)).toBe(
            "09 Apr 2028, 00:00 GMT+4 · Dubai PCG"
        )
    })

    it("writes voter times with the zone named in words", () => {
        expect(spaces(formatVoterDateTimeZone(opening, "Asia/Dubai", options()))).toBe(
            "Apr 9, 2028, 12:00 AM Gulf Standard Time"
        )
        expect(spaces(formatOnThisDevice(opening, options(), NEW_YORK))).toBe(
            "On this device: Apr 8, 2028, 4:00 PM"
        )
    })

    it("explains DST gaps and overlaps, and says nothing for other times", () => {
        const admin = options("en", adminFormat)
        expect(zonedTimeNote("2028-03-12T02:30", "America/Toronto", admin)).toBe(
            "12 Mar 2028, 02:30 does not exist in Toronto because clocks go forward. It will run at the time shown."
        )
        expect(zonedTimeNote("2028-11-05T01:30", "America/Toronto", admin)).toBe(
            "05 Nov 2028, 01:30 happens twice in Toronto. The first one is used."
        )
        expect(zonedTimeNote(OPENING, "Asia/Dubai", admin)).toBeUndefined()
    })

    it("falls back to the bundled defaults when the portal has no translations", () => {
        const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined)
        const bare = {t: ((key: string) => key) as unknown as TFunction, lang: "en"}
        expect(
            formatDateTimeZone(CLOSE, "Asia/Manila", {...bare, formatDateTime: adminFormat})
        ).toBe("08 May 2028, 19:00 PhST")
        expect(timeZoneOption("Asia/Manila", bare, CLOSE).label).toBe("(GMT+08:00) Manila")
        expect(warn).not.toHaveBeenCalled()
    })

    it("ships every timezone text in the eight languages with its placeholders", () => {
        for (const [lang, bundle] of Object.entries(BUNDLES)) {
            const texts = bundle.translations.timezones
            expect(texts.abbr["Asia/Manila"]).toBe("PhST")
            for (const [key, required] of Object.entries(TIME_ZONE_COMBINED_TEXT_PLACEHOLDERS)) {
                const name = key.replace("timezones.", "") as keyof typeof texts
                expect([
                    lang,
                    key,
                    missingTimeZonePlaceholders(key, texts[name] as string),
                ]).toEqual([lang, key, []])
                expect(required.length).toBeGreaterThan(0)
            }
            expect(texts.option).toContain("{{offset}}")
            expect(texts.option).toContain("{{city}}")
        }
    })
})

describe("overrides", () => {
    it("changes the Manila label everywhere it shows (PhST → PHT)", () => {
        override("en", {"timezones.abbr.Asia/Manila": "PHT"})
        expect(zoneLabel("Asia/Manila", options(), CLOSE)).toBe("PHT")
        expect(formatDateTimeZone(CLOSE, "Asia/Manila", options("en", adminFormat))).toBe(
            "08 May 2028, 19:00 PHT"
        )
        const zones = searchTimeZones("PHT", listTimeZones(), options())
        expect(zones.map((option) => option.zone)).toContain("Asia/Manila")
    })

    it("applies a global override and keeps other languages on their default", () => {
        overwriteTranslations(
            {i18n: {en: {"global:timezones.abbr.Asia/Manila": "PHT"}}},
            ADMIN_SCOPE
        )
        expect(zoneLabel("Asia/Manila", options(), CLOSE)).toBe("PHT")
        expect(zoneLabel("Asia/Manila", options("es"), CLOSE)).toBe("PhST")
    })

    it("changes long names, cities and the offset format", () => {
        override("en", {
            "timezones.name.Asia/Dubai": "UAE time",
            "timezones.city.Asia/Manila": "Metro Manila",
            "timezones.offset": "UTC{{sign}}{{hours}}{{minutes}}",
        })
        expect(formatVoterDateTimeZone(CLOSE, "Asia/Dubai", options("en", adminFormat))).toBe(
            "08 May 2028, 15:00 UAE time"
        )
        expect(timeZoneOption("Asia/Manila", options(), CLOSE).label).toBe(
            "(UTC+0800) Metro Manila"
        )
        expect(searchTimeZones("metro", listTimeZones(), options())[0].zone).toBe("Asia/Manila")
    })

    it("returns to the defaults once an override is removed", () => {
        override("en", {
            "timezones.name.Asia/Dubai": "UAE time",
            "timezones.abbr.Asia/Manila": "PHT",
        })
        overwriteTranslations({i18n: {}}, ADMIN_SCOPE)
        expect(zoneName("Asia/Dubai", options(), CLOSE)).toBe("Gulf Standard Time")
        expect(zoneLabel("Asia/Manila", options(), CLOSE)).toBe("PhST")
    })

    it("lets a language reword the picker option and detail", () => {
        override("tl", {
            "timezones.city.Asia/Manila": "Maynila",
            "timezones.optionDetail": "{{name}} ({{countries}})",
        })
        expect(timeZoneOption("Asia/Manila", options("tl"), CLOSE)).toMatchObject({
            label: "(GMT+08:00) Maynila",
            detail: `${intlName("Asia/Manila", "tl", CLOSE, "long")} (Pilipinas)`,
        })
    })

    it("reorders a combined string", () => {
        override("en", {
            "timezones.dateTimeZone": "{{zone}}: {{dateTime}}",
            "timezones.voterDateTimeZone": "{{dateTime}} ({{zoneName}})",
        })
        expect(formatDateTimeZone(CLOSE, "Asia/Manila", options("en", adminFormat))).toBe(
            "PhST: 08 May 2028, 19:00"
        )
        expect(formatVoterDateTimeZone(CLOSE, "Asia/Manila", options("en", adminFormat))).toBe(
            "08 May 2028, 19:00 (Philippine Standard Time)"
        )
    })

    it("falls back to the default with a warning when an override drops a placeholder", () => {
        const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined)
        override("en", {
            "timezones.dateTimeZone": "{{dateTime}}",
            "timezones.voterDateTimeZone": "{{zoneName}}",
            "timezones.gap": "Clocks go forward in {{city}}.",
        })
        const admin = options("en", adminFormat)
        expect(formatDateTimeZone(CLOSE, "Asia/Manila", admin)).toBe("08 May 2028, 19:00 PhST")
        expect(formatVoterDateTimeZone(CLOSE, "Asia/Manila", admin)).toBe(
            "08 May 2028, 19:00 Philippine Standard Time"
        )
        expect(zonedTimeNote("2028-03-12T02:30", "America/Toronto", admin)).toBe(
            "12 Mar 2028, 02:30 does not exist in Toronto because clocks go forward. It will run at the time shown."
        )
        expect(warn).toHaveBeenCalledTimes(3)
        expect(String(warn.mock.calls[0][0])).toContain("timezones.dateTimeZone")
    })

    it("names the placeholders a combined override is missing (Localization tabs)", () => {
        expect(missingTimeZonePlaceholders("global:timezones.myTime", "{{dateTime}} mine")).toEqual(
            ["zone"]
        )
        expect(
            missingTimeZonePlaceholders("timezones.voterDateTimeZone", "at {{dateTime}}")
        ).toEqual(["zoneName"])
        expect(
            missingTimeZonePlaceholders("votingPortal:timezones.gap", "{{ dateTime }} {{city}}")
        ).toEqual([])
        expect(missingTimeZonePlaceholders("timezones.onThisDevice", "Here")).toEqual(["dateTime"])
        // Not a combined string: nothing is required.
        expect(missingTimeZonePlaceholders("timezones.abbr.Asia/Manila", "PHT")).toEqual([])
    })
})

// The same screens by configuration only: the COMELEC preset and the Madrid
// association. Expectations come from the configuration, never from literals.
describe.each([
    {
        name: "COMELEC preset",
        primary: "Asia/Manila",
        configured: [
            "Asia/Manila",
            "Asia/Dubai",
            "Asia/Kolkata",
            "Asia/Kathmandu",
            "Asia/Tehran",
            "Asia/Yangon",
            "Africa/Cairo",
            "America/Toronto",
        ],
    },
    {
        name: "Madrid association",
        primary: "Europe/Madrid",
        configured: ["Europe/Madrid", "Atlantic/Canary"],
    },
])("$name", ({primary, configured}) => {
    it("opens every configured zone at local midnight and reads it back", () => {
        for (const zone of configured) {
            const {instant, kind} = zonedToInstant(OPENING, zone)
            expect([zone, kind]).toEqual([zone, "exact"])
            expect(instantToZoned(instant, zone)).toBe(OPENING)
            const offset = zoneOffsetMinutes(zone, new Date(instant))
            expect(Date.parse(instant)).toBe(Date.parse(`${OPENING}:00Z`) - offset * 60_000)
        }
    })

    it("finds every configured zone by its city first", () => {
        for (const zone of configured) {
            const city = zoneCity(zone, options())
            expect(searchTimeZones(city, configured, options())[0].zone).toBe(zone)
        }
    })

    it("labels the primary through its override, and only the primary", () => {
        const label = `${primary.split("/")[1]} time`
        override("en", {[`timezones.abbr.${primary}`]: label})
        for (const zone of configured) {
            const expected = zone === primary ? label : zoneLabel(zone, options(), CLOSE)
            expect(formatDateTimeZone(CLOSE, zone, options("en", adminFormat))).toBe(
                `${adminFormat(CLOSE, zone)} ${expected}`
            )
        }
        expect(zoneLabel(primary, options(), CLOSE)).toBe(label)
    })

    it("shows my time only through the browser zone", () => {
        const admin = options("en", adminFormat)
        const mine = zoneLabel(browserTimeZone(), admin, CLOSE)
        expect(formatMyTime(CLOSE, admin)).toBe(
            `${adminFormat(CLOSE, browserTimeZone())} ${mine} · my time`
        )
        expect(formatMyTime(CLOSE, admin, primary)).toBe(
            `${adminFormat(CLOSE, primary)} ${zoneLabel(primary, admin, CLOSE)} · my time`
        )
    })
})

describe("timezone localization when browser capabilities are unavailable", () => {
    it("lists supported country zones without Intl.supportedValuesOf", () => {
        const descriptor = Object.getOwnPropertyDescriptor(Intl, "supportedValuesOf")
        Object.defineProperty(Intl, "supportedValuesOf", {value: undefined, configurable: true})
        try {
            let service: typeof import("./timeZones") | undefined
            jest.isolateModules(() => {
                service = require("./timeZones")
            })
            const zones = (service as typeof import("./timeZones")).listTimeZones()
            expect(zones).toContain("Asia/Manila")
            expect(zones).toContain("UTC")
            expect(zones).not.toContain("Asia/Calcutta")
            expect(new Set(zones).size).toBe(zones.length)
        } finally {
            if (descriptor) Object.defineProperty(Intl, "supportedValuesOf", descriptor)
        }
    })

    it("accepts the standard Catalan language code for bundled offset labels", () => {
        const unavailable = {...options("ca-ES"), t: (() => null) as unknown as TFunction}
        expect(formatZoneOffset(345, unavailable)).toBe("GMT+05:45")
    })

    it("uses the current instant when picker callers omit a date", () => {
        jest.useFakeTimers().setSystemTime(CLOSE)
        try {
            expect(zoneLabel("UTC", options())).toBe("UTC")
            expect(zoneName("UTC", options())).toBe("Coordinated Universal Time")
            expect(timeZoneOption("Asia/Manila", options())).toEqual({
                zone: "Asia/Manila",
                label: "(GMT+08:00) Manila",
                detail: "Philippines · Philippine Standard Time",
                offsetMinutes: 480,
            })
        } finally {
            jest.useRealTimers()
        }
    })

    it.each(["", "Unsupported/Device"])(
        "uses UTC for the unavailable device zone %s",
        (timeZone) => {
            const resolved = new Intl.DateTimeFormat().resolvedOptions()
            jest.spyOn(Intl.DateTimeFormat.prototype, "resolvedOptions").mockReturnValue({
                ...resolved,
                timeZone,
            })
            expect(browserTimeZone()).toBe("UTC")
            expect(formatOnThisDevice(CLOSE, options("en", adminFormat))).toBe(
                "On this device: 08 May 2028, 11:00"
            )
        }
    )

    it.each(["en-US", "unknown-language", "bad_locale", ""])(
        "keeps English offset labels when the requested language %s has no bundle",
        (lang) => {
            const unavailable = {...options(lang), t: (() => null) as unknown as TFunction}
            expect(formatZoneOffset(330, unavailable)).toBe("GMT+05:30")
            expect(zoneName("Invalid/Zone", unavailable, CLOSE)).toBe("Invalid/Zone")
        }
    )

    it("keeps country codes in details if Intl region and list names fail", () => {
        jest.spyOn(Intl, "DisplayNames").mockImplementation(() => {
            throw new RangeError("Synthetic unsupported region names")
        })
        jest.spyOn(Intl, "ListFormat").mockImplementation(() => {
            throw new RangeError("Synthetic unsupported list formatting")
        })
        const unavailable = {...options("de-DE"), t: ((key: string) => key) as TFunction}
        const option = timeZoneOption("Europe/Zurich", unavailable, CLOSE)
        expect(option.label).toBe("(GMT+02:00) Zurich")
        expect(option.detail).toBe("CH · Mitteleuropäische Sommerzeit")
        expect(option.detail).toContain("Mitteleuropäische Sommerzeit")
    })
})
