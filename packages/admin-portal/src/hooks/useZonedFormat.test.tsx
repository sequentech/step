/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderHook} from "@testing-library/react"
import i18next, {type i18n as I18n} from "i18next"
import uiCoreEnglish from "../../../ui-core/src/translations/en"
import {zoneLabel} from "../../../ui-core/src/services/timeZones"
import {
    ELogTimeZonePolicy,
    type IElectionEventPresentation,
} from "../../../ui-core/src/types/ElectionEventPresentation"
import {
    effectiveTimeZone,
    formatMine,
    formatZoned,
    logTimeZone,
    primaryTimeZone,
} from "@/lib/timezones/zonedFormat"
import {useEventZonedFormat, useZonedFormat} from "./useZonedFormat"
import {EventTimeZoneProvider, MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"

// The timezone service and types from source, without the browser component barrel.
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../ui-core/src/services/timeZones"),
    ...jest.requireActual("../../../ui-core/src/services/eventTimeZones"),
    ...jest.requireActual("../../../ui-core/src/types/ElectionEventPresentation"),
}))

let mockI18n: I18n
jest.mock("react-i18next", () => ({useTranslation: () => ({t: mockI18n.t, i18n: mockI18n})}))

const newI18n = async (overrides: Record<string, string> = {}) => {
    const instance = i18next.createInstance()
    await instance.init({
        lng: "en",
        ns: ["translations"],
        defaultNS: "translations",
        interpolation: {escapeValue: false},
        showSupportNotice: false,
        // A copy: i18next keeps the object, and an override must not leak into the bundle.
        resources: {en: {translations: JSON.parse(JSON.stringify(uiCoreEnglish.translations))}},
    })
    for (const [key, value] of Object.entries(overrides)) {
        instance.addResource("en", "translations", key, value)
    }
    return instance
}

/** The two configurations: the COMELEC preset and the Madrid association. */
const CONFIGURATIONS: Array<[string, IElectionEventPresentation]> = [
    [
        "Manila primary, logs in the primary",
        {
            timezones: {
                configured: ["Asia/Manila", "Asia/Dubai", "America/Los_Angeles"],
                primary: "Asia/Manila",
                logs: ELogTimeZonePolicy.PRIMARY,
            },
        } as IElectionEventPresentation,
    ],
    [
        "Madrid primary, logs in each election's zone",
        {
            timezones: {
                configured: ["Europe/Madrid", "Atlantic/Canary"],
                primary: "Europe/Madrid",
                logs: ELogTimeZonePolicy.ELECTION,
            },
        } as IElectionEventPresentation,
    ],
]

// 2028-04-08T22:00:03Z: 06:00:03 in Manila, 00:00:03 in Madrid, 18:00:03 EDT in New York.
const AT = new Date("2028-04-08T22:00:03Z")
/** "My time" is explicit: the tests never depend on the machine's zone. */
const MY_ZONE = "America/New_York"

beforeEach(async () => {
    mockI18n = await newI18n()
})

const inScreen =
    (event?: {id: string; presentation: IElectionEventPresentation}) =>
    ({children}: {children: React.ReactNode}) => (
        <MyTimeZoneProvider zone={MY_ZONE}>
            <EventTimeZoneProvider event={event}>{children}</EventTimeZoneProvider>
        </MyTimeZoneProvider>
    )

describe("which zone a time is shown in", () => {
    it.each(CONFIGURATIONS)("%s", (_name, presentation) => {
        const {configured, primary, logs} = presentation.timezones!
        const other = configured.find((zone) => zone !== primary)!
        expect(primaryTimeZone(presentation)).toBe(primary)
        expect(effectiveTimeZone(presentation, {timezone: other} as never)).toBe(other)
        expect(effectiveTimeZone(presentation, {timezone: "Asia/Tokyo"} as never)).toBe(primary)
        expect(logTimeZone(presentation, {timezone: other} as never)).toBe(
            logs === ELogTimeZonePolicy.PRIMARY ? primary : other
        )
        // Event-wide rows: the primary.
        expect(logTimeZone(presentation, undefined)).toBe(primary)
    })

    it("falls back to UTC without configured zones, and logs default to each election's zone", () => {
        expect(primaryTimeZone(undefined)).toBe("UTC")
        expect(logTimeZone({} as IElectionEventPresentation, undefined)).toBe("UTC")
        const presentation = {
            timezones: {configured: ["Europe/Madrid", "Atlantic/Canary"], primary: "Europe/Madrid"},
        } as IElectionEventPresentation
        expect(logTimeZone(presentation, {timezone: "Atlantic/Canary"} as never)).toBe(
            "Atlantic/Canary"
        )
    })
})

describe("one value with a label", () => {
    it.each(CONFIGURATIONS)("%s: the primary zone's label", (_name, presentation) => {
        const primary = presentation.timezones!.primary
        const text = formatZoned(AT, primary, {t: mockI18n.t, lang: "en", seconds: true})
        const label = zoneLabel(primary, {t: mockI18n.t, lang: "en"}, AT)
        expect(text.endsWith(` ${label}`)).toBe(true)
        expect(text).toContain(
            new Intl.DateTimeFormat("en", {timeStyle: "medium", timeZone: primary}).format(AT)
        )
    })

    it("uses the zone label key, so a tenant override changes it", async () => {
        const options = {t: mockI18n.t, lang: "en"}
        expect(formatZoned(AT, "Asia/Manila", options)).toMatch(/ PhST$/)
        mockI18n = await newI18n({"timezones.abbr.Asia/Manila": "PHT"})
        expect(formatZoned(AT, "Asia/Manila", {t: mockI18n.t, lang: "en"})).toMatch(/ PHT$/)
    })

    it("adds my time only when I am in another zone", () => {
        const options = {t: mockI18n.t, lang: "en", seconds: true}
        const mine = formatMine(AT, "Asia/Manila", options, MY_ZONE)
        expect(mine).toContain("6:00:03 PM EDT · my time")
        expect(formatMine(AT, MY_ZONE, options, MY_ZONE)).toBeNull()
        expect(formatMine(null, "Asia/Manila", options, MY_ZONE)).toBeNull()
        expect(formatZoned(null, "Asia/Manila", options)).toBe("")
    })
})

describe("useZonedFormat", () => {
    it("shows values in the given zone, else mine", () => {
        const {result} = renderHook(() => useZonedFormat("Asia/Manila"), {wrapper: inScreen()})
        expect(result.current.zone).toBe("Asia/Manila")
        expect(result.current.format(AT)).toMatch(/PhST$/)
        expect(result.current.format(AT, "Europe/Madrid")).toMatch(/GMT\+2$/)
        expect(result.current.mine(AT)).toContain("EDT · my time")
        const mine = renderHook(() => useZonedFormat(null), {wrapper: inScreen()}).result.current
        expect(mine.zone).toBe(MY_ZONE)
        expect(mine.mine(AT)).toBeNull()
    })

    it.each(CONFIGURATIONS)("%s: an event's times are in its primary", (_name, presentation) => {
        const event = {id: "event-id", presentation}
        const fromScreen = renderHook(() => useEventZonedFormat("event-id", {seconds: true}), {
            wrapper: inScreen(event),
        }).result.current
        expect(fromScreen.zone).toBe(presentation.timezones!.primary)
        expect(fromScreen.mine(AT)).toContain("· my time")
        // A record the caller has wins; another event's id isn't guessed.
        const fromRecord = renderHook(() => useEventZonedFormat(event), {
            wrapper: inScreen(),
        }).result.current
        expect(fromRecord.zone).toBe(presentation.timezones!.primary)
        const other = renderHook(() => useEventZonedFormat("other-event"), {
            wrapper: inScreen(event),
        }).result.current
        expect(other.zone).toBe(MY_ZONE)
    })

    it.each(CONFIGURATIONS)("%s: an election's screens use its zone", (_name, presentation) => {
        const other = presentation.timezones!.configured.find(
            (zone) => zone !== presentation.timezones!.primary
        )!
        const wrapper = ({children}: {children: React.ReactNode}) => (
            <MyTimeZoneProvider zone={MY_ZONE}>
                <EventTimeZoneProvider
                    event={{id: "event-id", presentation}}
                    election={{presentation: {timezone: other}}}
                >
                    {children}
                </EventTimeZoneProvider>
            </MyTimeZoneProvider>
        )
        const {result} = renderHook(() => useEventZonedFormat("event-id"), {wrapper})
        expect(result.current.zone).toBe(other)
    })

    it("outside an event's screens, times are in my zone", () => {
        const {result} = renderHook(() => useEventZonedFormat(null), {wrapper: inScreen()})
        expect(result.current.zone).toBe(MY_ZONE)
    })
})
