// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import i18next from "i18next"
import {
    EVotingPortalDateTimeFormat,
    IElectionEventPresentation,
    formatVotingPortalDateTime,
} from "@sequentech/ui-core"
import english from "../translations/en"
import core from "../../../ui-core/src/translations/en"
import {
    ASSOCIATION,
    IZonedConfiguration,
    OVERSEAS,
    TEST_VOTING_DUBAI,
    zonedElectionDates,
} from "../../../ui-essentials/src/components/SelectElection/__stories__/zonedFixtures"
import {ballotTimeZones, formatBallotTime, votingClosedMessage} from "./ElectionTimeZones"

const eventOf = (config: IZonedConfiguration) =>
    ({
        timezones: {configured: config.configured, primary: config.primary},
    }) as Pick<IElectionEventPresentation, "timezones">

const EVENT = {
    id: "event-closed",
    presentation: {voting_portal_datetime_format: EVotingPortalDateTimeFormat.ISO_LOCAL},
}
const format = (input: string, zone?: string) =>
    formatVotingPortalDateTime(input, EVENT as never, "en", zone)

const t = i18next.createInstance()
beforeAll(async () => {
    await t.init({
        lng: "en",
        resources: {
            en: {translation: {...english.translations, timezones: core.translations.timezones}},
        },
        interpolation: {escapeValue: false},
    })
})

// The dates as get_election_dates writes them: the Post's opening in its zone,
// the event-wide close in the primary.
const datesOf = (config: IZonedConfiguration, post: {opensAt: string; timeZone: string}) =>
    zonedElectionDates(post.opensAt, config.closesAt, {
        openZone: post.timeZone,
        closeZone: config.primary,
    })

const message = (
    config: IZonedConfiguration | null,
    ballots: Array<{timeZone: string; dates: ReturnType<typeof zonedElectionDates>}>,
    now: string
) =>
    votingClosedMessage({
        ballots: ballots.map(({timeZone, dates}) => ({
            electionDates: dates,
            zones: ballotTimeZones(config && eventOf(config), {timezone: timeZone}),
        })),
        now: new Date(now),
        t: t.t,
        lang: "en",
        formatDateTime: (input, zone) => format(input, zone),
    })

describe("ballotTimeZones", () => {
    it("is the Post's zone and the primary, by the sequent-core rule", () => {
        for (const config of [OVERSEAS, ASSOCIATION]) {
            for (const post of config.posts) {
                expect(ballotTimeZones(eventOf(config), {timezone: post.timeZone})).toEqual({
                    timeZone: post.timeZone,
                    closeTimeZone: config.primary,
                })
            }
            expect(ballotTimeZones(eventOf(config), {timezone: "Asia/Tokyo"})).toEqual({
                timeZone: config.primary,
                closeTimeZone: config.primary,
            })
        }
    })

    it("falls back to the ballot style's copies when the published records carry no zones", () => {
        expect(
            ballotTimeZones(
                {},
                {},
                {
                    election_event_presentation: eventOf(OVERSEAS),
                    election_presentation: {timezone: "Asia/Dubai"},
                }
            )
        ).toEqual({timeZone: "Asia/Dubai", closeTimeZone: "Asia/Manila"})
    })

    it("is nothing for an event without timezones, so its dates render as before", () => {
        expect(ballotTimeZones(undefined, undefined)).toBeUndefined()
        expect(ballotTimeZones({}, {timezone: "Asia/Dubai"}, {})).toBeUndefined()
        expect(
            ballotTimeZones({timezones: {configured: [], primary: " "}}, {timezone: "Asia/Dubai"})
        ).toBeUndefined()
    })
})

describe("formatBallotTime (Ballot Locator)", () => {
    const instant = new Date(OVERSEAS.closesAt)

    it("names the Post's zone", () => {
        expect(
            formatBallotTime(instant, {
                event: EVENT as never,
                zones: ballotTimeZones(eventOf(OVERSEAS), {timezone: "Asia/Dubai"}),
                t: t.t,
                lang: "en",
            })
        ).toBe("2028-05-08 15:00 Gulf Standard Time")
    })

    it("renders as before for an event without timezones", () => {
        expect(
            formatBallotTime(instant, {
                event: EVENT as never,
                zones: ballotTimeZones({}, {timezone: "Asia/Dubai"}),
                t: t.t,
                lang: "en",
            })
        ).toBe(format(OVERSEAS.closesAt))
    })
})

describe("votingClosedMessage", () => {
    const [DUBAI, NAIROBI] = OVERSEAS.posts
    const [CENTRAL, CANARY] = ASSOCIATION.posts

    it("gives the close in the primary with the Post's time (the closed draft)", () => {
        expect(
            message(
                OVERSEAS,
                [
                    {timeZone: DUBAI.timeZone, dates: datesOf(OVERSEAS, DUBAI)},
                    {timeZone: DUBAI.timeZone, dates: TEST_VOTING_DUBAI},
                ],
                OVERSEAS.now.closed
            )
        ).toBe(
            "Voting closed on 2028-05-08 19:00 Philippine Standard Time (2028-05-08 15:00 Gulf Standard Time)."
        )
        expect(
            message(
                OVERSEAS,
                [{timeZone: NAIROBI.timeZone, dates: datesOf(OVERSEAS, NAIROBI)}],
                OVERSEAS.now.closed
            )
        ).toBe(
            "Voting closed on 2028-05-08 19:00 Philippine Standard Time (2028-05-08 14:00 East Africa Time)."
        )
    })

    it("names the close once when the Post's clock reads the same as the primary's", () => {
        expect(
            message(
                ASSOCIATION,
                [{timeZone: CENTRAL.timeZone, dates: datesOf(ASSOCIATION, CENTRAL)}],
                ASSOCIATION.now.closed
            )
        ).toBe("Voting closed on 2028-06-09 20:00 Central European Summer Time.")
        expect(
            message(
                ASSOCIATION,
                [{timeZone: CANARY.timeZone, dates: datesOf(ASSOCIATION, CANARY)}],
                ASSOCIATION.now.closed
            )
        ).toBe(
            "Voting closed on 2028-06-09 20:00 Central European Summer Time (2028-06-09 19:00 Western European Summer Time)."
        )
    })

    it("a Post's own close, the latest one, shows once in the Post's zone", () => {
        expect(
            message(
                OVERSEAS,
                [{timeZone: DUBAI.timeZone, dates: TEST_VOTING_DUBAI}],
                "2028-04-08T20:30:00.000Z"
            )
        ).toBe("Voting closed on 2028-04-08 23:59 Gulf Standard Time.")
    })

    it("says nothing while a listed ballot's close is still ahead", () => {
        // The test voting closed on 08 Apr; the election opens on 09 Apr.
        expect(
            message(
                OVERSEAS,
                [
                    {timeZone: DUBAI.timeZone, dates: datesOf(OVERSEAS, DUBAI)},
                    {timeZone: DUBAI.timeZone, dates: TEST_VOTING_DUBAI},
                ],
                "2028-04-08T19:59:30.000Z"
            )
        ).toBeUndefined()
        expect(
            message(
                ASSOCIATION,
                ASSOCIATION.posts.map((post) => ({
                    timeZone: post.timeZone,
                    dates: datesOf(ASSOCIATION, post),
                })),
                ASSOCIATION.now.open
            )
        ).toBeUndefined()
    })

    it("says nothing without a close date, or for an event without timezones", () => {
        expect(
            message(OVERSEAS, [{timeZone: "Asia/Dubai", dates: {}}], OVERSEAS.now.closed)
        ).toBeUndefined()
        expect(
            message(
                null,
                [{timeZone: DUBAI.timeZone, dates: datesOf(OVERSEAS, DUBAI)}],
                OVERSEAS.now.closed
            )
        ).toBeUndefined()
    })
})
