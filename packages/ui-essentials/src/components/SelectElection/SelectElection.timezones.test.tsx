/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render} from "@testing-library/react"
import {ThemeProvider} from "@mui/material"
import i18next, {i18n as I18n} from "i18next"
import {I18nextProvider} from "react-i18next"
import {EVotingPortalDateTimeFormat, formatVotingPortalDateTime} from "@sequentech/ui-core"
import SelectElection from "./SelectElection"
import theme from "../../services/theme"
import english from "../../translations/en"
import core from "../../../../ui-core/src/translations/en"
import {
    ASSOCIATION,
    IZonedConfiguration,
    OVERSEAS,
    TEST_VOTING_DUBAI,
    zonedElectionDates,
} from "./__stories__/zonedFixtures"

jest.mock("../LinkBehavior/LinkBehavior", () => "a")

const EVENT = {
    id: "event-zones",
    presentation: {voting_portal_datetime_format: EVotingPortalDateTimeFormat.LOCALE_MEDIUM},
}
const format = (input: string, zone?: string) =>
    formatVotingPortalDateTime(input, EVENT as never, "en", zone).replace(/\u202f/g, " ")

// Independent of the component: Intl's own date, time and zone name.
const intlDateTime = (instant: string, zone: string) =>
    new Intl.DateTimeFormat("en", {dateStyle: "medium", timeStyle: "short", timeZone: zone})
        .format(new Date(instant))
        .replace(/\u202f/g, " ")
const intlZoneName = (instant: string, zone: string) =>
    new Intl.DateTimeFormat("en", {timeZone: zone, timeZoneName: "long"})
        .formatToParts(new Date(instant))
        .find((part) => part.type === "timeZoneName")!.value
const voterText = (instant: string, zone: string) =>
    `${intlDateTime(instant, zone)} ${intlZoneName(instant, zone)}`

const makeI18n = async (overrides: Record<string, unknown> = {}): Promise<I18n> => {
    const instance = i18next.createInstance()
    await instance.init({
        lng: "en",
        resources: {
            en: {
                translation: {
                    ...english.translations,
                    timezones: {...core.translations.timezones, ...overrides},
                },
            },
        },
        interpolation: {escapeValue: false},
    })
    return instance
}

const show = async (
    props: Partial<React.ComponentProps<typeof SelectElection>> &
        Pick<React.ComponentProps<typeof SelectElection>, "electionDates">,
    i18n?: I18n
) => {
    const {container} = render(
        <I18nextProvider i18n={i18n ?? (await makeI18n())}>
            <ThemeProvider theme={theme}>
                <SelectElection
                    isActive
                    isOpen
                    isStarted
                    hasVoted={false}
                    title="Ballot"
                    formatDateTime={format}
                    {...props}
                />
            </ThemeProvider>
        </I18nextProvider>
    )
    const text = (selector: string) =>
        container.querySelector(selector)?.textContent?.replace(/\u202f/g, " ") ?? null
    return {
        open: text(".election-open-date-value"),
        close: text(".election-close-date-value"),
        closeLocal: text(".election-close-date-local"),
        device: text(".election-device-time"),
        countdown: text(".election-countdown"),
    }
}

const at = (instant: string) => jest.setSystemTime(new Date(instant))

beforeEach(() => jest.useFakeTimers())
afterEach(() => jest.useRealTimers())

describe.each<[string, IZonedConfiguration]>([
    ["overseas event", OVERSEAS],
    ["staff association", ASSOCIATION],
])("SelectElection with zones — %s", (_name, config) => {
    const {primary, closesAt} = config

    it.each(config.posts.map((post) => [post.title, post] as const))(
        "%s: opening in the Post's zone, close in the primary, the Post's close below when it differs",
        async (_title, post) => {
            at(config.now.open)
            const lines = await show({
                electionDates: zonedElectionDates(post.opensAt, closesAt),
                timeZone: post.timeZone,
                closeTimeZone: primary,
                deviceTimeZone: post.timeZone,
            })
            expect(lines.open).toBe(voterText(post.opensAt, post.timeZone))
            expect(lines.close).toBe(voterText(closesAt, primary))
            const sameClock =
                intlDateTime(closesAt, post.timeZone) === intlDateTime(closesAt, primary)
            expect(lines.closeLocal).toBe(sameClock ? null : voterText(closesAt, post.timeZone))
            expect(lines.device).toBeNull()
        }
    )

    it("before the opening: the countdown, and the opening on a device in another zone", async () => {
        const post = config.posts[config.posts.length - 1]
        at(config.now.before)
        const device = "America/New_York"
        const lines = await show({
            isOpen: false,
            electionDates: zonedElectionDates(post.opensAt, closesAt),
            timeZone: post.timeZone,
            closeTimeZone: primary,
            deviceTimeZone: device,
        })
        expect(lines.countdown).toMatch(/Election Begins in/)
        expect(lines.device).toBe(`On this device: ${format(post.opensAt, device)}`)
    })

    it("once open: the device line moves to the close", async () => {
        const post = config.posts[0]
        at(config.now.open)
        const device = "America/New_York"
        const lines = await show({
            electionDates: zonedElectionDates(post.opensAt, closesAt),
            timeZone: post.timeZone,
            closeTimeZone: primary,
            deviceTimeZone: device,
        })
        expect(lines.countdown).toBeNull()
        expect(lines.device).toBe(`On this device: ${format(closesAt, device)}`)
    })
})

describe("SelectElection with zones — the drafts", () => {
    const dubai = OVERSEAS.posts[0]
    const nairobi = OVERSEAS.posts[1]

    it("the ballot list (desktop and phone)", async () => {
        at(OVERSEAS.now.open)
        const lines = await show({
            electionDates: zonedElectionDates(dubai.opensAt, OVERSEAS.closesAt),
            timeZone: dubai.timeZone,
            closeTimeZone: OVERSEAS.primary,
            deviceTimeZone: dubai.timeZone,
        })
        expect(lines).toEqual({
            open: "Apr 9, 2028, 12:00 AM Gulf Standard Time",
            close: "May 8, 2028, 7:00 PM Philippine Standard Time",
            closeLocal: "May 8, 2028, 3:00 PM Gulf Standard Time",
            device: null,
            countdown: null,
        })
    })

    it("a Post's own close (test voting) shows in the zone it was scheduled in, one line", async () => {
        at(OVERSEAS.now.open)
        // The props as the ballot list passes them: the close zone is the primary.
        const lines = await show({
            isOpen: false,
            hasVoted: true,
            electionDates: TEST_VOTING_DUBAI,
            timeZone: dubai.timeZone,
            closeTimeZone: OVERSEAS.primary,
            deviceTimeZone: dubai.timeZone,
        })
        expect(lines.open).toBe("Feb 9, 2028, 12:00 AM Gulf Standard Time")
        expect(lines.close).toBe("Apr 8, 2028, 11:59 PM Gulf Standard Time")
        expect(lines.closeLocal).toBeNull()
    })

    it("an event-wide close that names the primary shows in it, with the Post's time below", async () => {
        at(OVERSEAS.now.open)
        const lines = await show({
            electionDates: zonedElectionDates(dubai.opensAt, OVERSEAS.closesAt, {
                openZone: dubai.timeZone,
                closeZone: OVERSEAS.primary,
            }),
            timeZone: dubai.timeZone,
            closeTimeZone: OVERSEAS.primary,
            deviceTimeZone: dubai.timeZone,
        })
        expect(lines.close).toBe("May 8, 2028, 7:00 PM Philippine Standard Time")
        expect(lines.closeLocal).toBe("May 8, 2028, 3:00 PM Gulf Standard Time")
    })

    it("a device in another zone (Nairobi PE, before the opening)", async () => {
        at(OVERSEAS.now.before)
        const lines = await show({
            isOpen: false,
            electionDates: zonedElectionDates(nairobi.opensAt, OVERSEAS.closesAt),
            timeZone: nairobi.timeZone,
            closeTimeZone: OVERSEAS.primary,
            deviceTimeZone: "Europe/Berlin",
        })
        expect(lines.open).toBe("Apr 9, 2028, 12:00 AM East Africa Time")
        expect(lines.closeLocal).toBe("May 8, 2028, 2:00 PM East Africa Time")
        expect(lines.device).toBe("On this device: Apr 8, 2028, 11:00 PM")
    })

    it("shows the device time for a distinct zone even when its offset matches (Muscat for Dubai)", async () => {
        at(OVERSEAS.now.before)
        const lines = await show({
            isOpen: false,
            electionDates: zonedElectionDates(dubai.opensAt, OVERSEAS.closesAt),
            timeZone: dubai.timeZone,
            closeTimeZone: OVERSEAS.primary,
            deviceTimeZone: "Asia/Muscat",
        })
        expect(lines.device).toBe(`On this device: ${format(dubai.opensAt, "Asia/Muscat")}`)
    })

    it("keeps one line for canonical aliases of the Post timezone", async () => {
        at(OVERSEAS.now.before)
        const lines = await show({
            isOpen: false,
            electionDates: zonedElectionDates(dubai.opensAt, OVERSEAS.closesAt),
            timeZone: "Asia/Kolkata",
            closeTimeZone: OVERSEAS.primary,
            deviceTimeZone: "Asia/Calcutta",
        })
        expect(lines.device).toBeNull()
    })

    it("overrides of timezones.name.Asia/Dubai and timezones.voterDateTimeZone change the list", async () => {
        at(OVERSEAS.now.open)
        const i18n = await makeI18n({
            name: {"Asia/Dubai": "Dubai time"},
            voterDateTimeZone: "{{dateTime}} ({{zoneName}})",
        })
        const lines = await show(
            {
                electionDates: zonedElectionDates(dubai.opensAt, OVERSEAS.closesAt),
                timeZone: dubai.timeZone,
                closeTimeZone: OVERSEAS.primary,
                deviceTimeZone: dubai.timeZone,
            },
            i18n
        )
        expect(lines.open).toBe("Apr 9, 2028, 12:00 AM (Dubai time)")
        expect(lines.close).toBe("May 8, 2028, 7:00 PM (Philippine Standard Time)")
        expect(lines.closeLocal).toBe("May 8, 2028, 3:00 PM (Dubai time)")
    })

    it("without a zone, the dates render as before (browser zone, no zone names)", async () => {
        at(OVERSEAS.now.open)
        const lines = await show({
            electionDates: zonedElectionDates(dubai.opensAt, OVERSEAS.closesAt),
        })
        expect(lines.open).toBe(format(dubai.opensAt))
        expect(lines.close).toBe(format(OVERSEAS.closesAt))
        expect(lines.closeLocal).toBeNull()
        expect(lines.device).toBeNull()
    })
})

it("uses authenticated deadline authority instead of a rejected earlier published close", async () => {
    const {getEndDateEntry} = await import("./electionTimes")
    const legacy = {
        scheduled_event_dates: {END_VOTING_PERIOD: {scheduled_at: "2030-01-02T09:00:00Z"}},
    }
    expect(
        getEndDateEntry({
            ...legacy,
            authoritative_close: {scheduled_at: "2030-01-02T10:00:00Z", timezone: "Asia/Manila"},
        })
    ).toEqual({date: "2030-01-02T10:00:00Z", timeZone: "Asia/Manila"})
    expect(getEndDateEntry({...legacy, authoritative_close: {scheduled_at: null}})).toBeNull()
    expect(getEndDateEntry(legacy)?.date).toBe("2030-01-02T09:00:00Z")
})

it("uses the recorded stop only for a closed display, preserving its scheduled deadline", async () => {
    const {getEndDateEntry} = await import("./electionTimes")
    const dates = {
        last_started_at: "2030-01-02T08:00:00Z",
        last_stopped_at: "2030-01-02T09:00:00Z",
        authoritative_close: {scheduled_at: "2030-01-02T10:00:00Z", timezone: "Asia/Manila"},
    }
    expect(getEndDateEntry(dates)).toEqual({date: "2030-01-02T10:00:00Z", timeZone: "Asia/Manila"})
    expect(getEndDateEntry(dates, new Date("2030-01-02T09:30:00Z"))).toEqual({
        date: "2030-01-02T09:00:00Z",
    })
    expect(
        getEndDateEntry(
            {...dates, authoritative_close: {scheduled_at: null}},
            new Date("2030-01-02T09:30:00Z")
        )
    ).toEqual({date: "2030-01-02T09:00:00Z"})
    expect(
        getEndDateEntry(
            {...dates, last_started_at: "2030-01-02T09:15:00Z"},
            new Date("2030-01-02T09:30:00Z")
        )
    ).toEqual({date: "2030-01-02T10:00:00Z", timeZone: "Asia/Manila"})
    expect(getEndDateEntry(dates, new Date("2030-01-02T10:30:00Z"))).toEqual({
        date: "2030-01-02T09:00:00Z",
    })
    expect(
        getEndDateEntry(
            {...dates, last_stopped_at: "2030-01-02T10:01:00Z"},
            new Date("2030-01-02T10:30:00Z")
        )
    ).toEqual({date: "2030-01-02T10:00:00Z", timeZone: "Asia/Manila"})
})
