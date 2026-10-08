// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Meta, StoryContext, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {
    ETranslationScope,
    EVotingPortalDateTimeFormat,
    browserTimeZone,
    formatVotingPortalDateTime,
    overwriteTranslations,
} from "@sequentech/ui-core"
import {sameWallClock} from "@sequentech/ui-essentials"
import {
    ScenarioId,
    type JsonObject,
    type PreviewBallotStyle,
    type PreviewElection,
    type ScenarioSnapshot,
} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {PreviewScreen} from "../../preview/screens"
import {scenarioMeta, screenStory} from "../../preview/__stories__/scenarioStories"
import {
    ASSOCIATION,
    IZonedConfiguration,
    IZonedPost,
    OVERSEAS,
    TEST_VOTING_DUBAI,
    zonedElectionDates,
} from "../../../../ui-essentials/src/components/SelectElection/__stories__/zonedFixtures"

/**
 * The ballot list with timezones (VOTE-LIFECYCLE drafts `tz-vp-*` and
 * `tz-localization-voter`): the production chooser, loaded through the voter
 * preview with an event whose Posts are in their own zones. Every story freezes
 * the clock at the draft's moment and reads the dates in the Post's zone.
 */
export default {title: "Voting/Ballot list timezones", ...scenarioMeta} satisfies Meta

type State = "before" | "open" | "closed"

const FORMAT = EVotingPortalDateTimeFormat.LOCALE_MEDIUM
const STATUS: Record<State, string> = {before: "NOT_STARTED", open: "OPEN", closed: "CLOSED"}
const TEST_ELECTION_ID = "30000000-0000-4000-8000-0000000000aa"

interface ZonedStory {
    config: IZonedConfiguration
    post: IZonedPost
    state: State
    /** Adds the Post's earlier test voting, as in the drafts. */
    testVoting?: boolean
}

const withStatus = (status: JsonObject, state: State): JsonObject => ({
    ...status,
    voting_status: STATUS[state],
    kiosk_voting_status: STATUS[state],
    early_voting_status: "CLOSED",
})

const named = (presentation: JsonObject, name: string, timeZone: string): JsonObject => ({
    ...presentation,
    i18n: {en: {name, description: ""}},
    timezone: timeZone,
})

/** The scenario event with zones, one Post's ballot and optionally its test voting. */
const zonedSnapshot =
    ({config, post, state, testVoting}: ZonedStory) =>
    (snapshot: ScenarioSnapshot): ScenarioSnapshot => {
        const {preview} = snapshot
        const eventPresentation: JsonObject = {
            ...(preview.election_event.presentation as JsonObject),
            timezones: {configured: config.configured, primary: config.primary, logs: "election"},
            voting_portal_datetime_format: FORMAT,
        }
        preview.election_event = {
            ...preview.election_event,
            presentation: eventPresentation,
            status: withStatus(preview.election_event.status as JsonObject, state),
        }
        const [election] = preview.elections
        const [style] = preview.ballot_styles
        const ballot = (
            id: string,
            name: string,
            dates: JsonObject,
            ballotState: State
        ): [PreviewElection, PreviewBallotStyle] => {
            const presentation = named(election.presentation as JsonObject, name, post.timeZone)
            return [
                {
                    ...election,
                    id,
                    presentation,
                    status: withStatus(election.status as JsonObject, ballotState),
                },
                {
                    ...style,
                    id: `${style.id}-${id}`,
                    election_id: id,
                    contests: style.contests.map((contest) => ({...contest, election_id: id})),
                    election_presentation: presentation,
                    election_event_presentation: eventPresentation,
                    election_dates: dates,
                },
            ]
        }
        const ballots = [
            ballot(
                election.id,
                post.title,
                zonedElectionDates(post.opensAt, config.closesAt, {
                    openZone: post.timeZone,
                    closeZone: config.primary,
                }) as JsonObject,
                state
            ),
            ...(testVoting
                ? [
                      ballot(
                          TEST_ELECTION_ID,
                          `${post.title} · Test voting`,
                          TEST_VOTING_DUBAI as JsonObject,
                          "closed"
                      ),
                  ]
                : []),
        ]
        preview.elections = ballots.map(([item]) => item)
        preview.ballot_styles = ballots.map(([, item]) => item)
        return snapshot
    }

/** Freezes `Date` at `instant` while the story runs, as the drafts do. */
const freezeClock = (instant: string) => () => {
    const clock = Date.parse(instant)
    const OriginalDate = Date
    globalThis.Date = new Proxy(OriginalDate, {
        construct: (target, args) => Reflect.construct(target, args.length ? args : [clock]),
        apply: () => new OriginalDate(clock).toString(),
        get: (target, property, receiver) =>
            property === "now" ? () => clock : Reflect.get(target, property, receiver),
    })
    return () => {
        globalThis.Date = OriginalDate
    }
}

const PHONE = {viewport: {value: "iphone6", isRotated: false}}

const zonedStory = (
    story: ZonedStory & {
        phone?: boolean
        overrides?: Record<string, Record<string, string>>
        play: (context: StoryContext) => Promise<void>
    }
): StoryObj => {
    const base = screenStory(ScenarioId.SIMPLE_PLURALITY, PreviewScreen.CHOOSER, {
        chrome: true,
        snapshot: zonedSnapshot(story),
        play: story.play,
    })
    return {
        ...base,
        globals: story.phone ? PHONE : undefined,
        beforeEach: () => {
            const restoreClock = freezeClock(story.config.now[story.state])()
            if (!story.overrides) {
                return restoreClock
            }
            // As TenantEvent applies the event's overrides in production.
            const scope = {
                scope: ETranslationScope.VOTING_PORTAL,
                legacyScope: ETranslationScope.VOTING_PORTAL,
                changeDefaultLanguage: false,
            }
            overwriteTranslations({i18n: story.overrides}, scope)
            return () => {
                overwriteTranslations(undefined, scope)
                restoreClock()
            }
        },
    }
}

const card = async (canvasElement: HTMLElement, title: string) => {
    const heading = await within(canvasElement).findByRole("heading", {name: title})
    return within(heading.closest(".election-item") as HTMLElement)
}

const [DUBAI, NAIROBI] = OVERSEAS.posts
const [CENTRAL, CANARY] = ASSOCIATION.posts

const playChooser = async ({canvasElement}: StoryContext) => {
    const dubai = await card(canvasElement, DUBAI.title)
    await expect(dubai.getByText(/Apr 9, 2028, 12:00\sAM Gulf Standard Time/)).toBeVisible()
    await expect(dubai.getByText(/May 8, 2028, 7:00\sPM Philippine Standard Time/)).toBeVisible()
    await expect(dubai.getByText(/May 8, 2028, 3:00\sPM Gulf Standard Time/)).toBeVisible()
}

/** `tz-vp-chooser`: open; the close in the primary with the Post's time below. */
export const Chooser = zonedStory({
    config: OVERSEAS,
    post: DUBAI,
    state: "open",
    testVoting: true,
    play: playChooser,
})

/** `tz-vp-chooser-mobile`. */
export const ChooserPhone = zonedStory({
    config: OVERSEAS,
    post: DUBAI,
    state: "open",
    testVoting: true,
    phone: true,
    play: playChooser,
})

/** `tz-vp-before-mobile`: before the opening, with the countdown. */
export const BeforeOpeningPhone = zonedStory({
    config: OVERSEAS,
    post: DUBAI,
    state: "before",
    testVoting: true,
    phone: true,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(/Election Begins in/)).toBeVisible()
    },
})

/** `tz-vp-closed-mobile`: after the close, the closed message. */
export const ClosedPhone = zonedStory({
    config: OVERSEAS,
    post: DUBAI,
    state: "closed",
    testVoting: true,
    phone: true,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                /Voting closed on May 8, 2028, 7:00\sPM Philippine Standard Time \(May 8, 2028, 3:00\sPM Gulf Standard Time\)\./
            )
        ).toBeVisible()
    },
})

/**
 * `tz-vp-device-mobile`: the device's zone isn't the Post's. The device is the
 * browser (UTC in the story tests), so the expectation is derived from the
 * browser's zone, never assumed: the line shows the opening on the device when
 * that zone reads differently from Nairobi's, and is absent otherwise. The
 * component tests cover explicit device zones.
 */
export const OtherDevicePhone = zonedStory({
    config: OVERSEAS,
    post: NAIROBI,
    state: "before",
    testVoting: false,
    phone: true,
    play: async ({canvasElement}) => {
        const nairobi = await card(canvasElement, NAIROBI.title)
        await expect(nairobi.getByText(/Apr 9, 2028, 12:00\sAM East Africa Time/)).toBeVisible()
        const device = browserTimeZone()
        if (sameWallClock(NAIROBI.opensAt, NAIROBI.timeZone, device)) {
            await expect(nairobi.queryByText(/^On this device: /)).not.toBeInTheDocument()
        } else {
            const onDevice = formatVotingPortalDateTime(
                NAIROBI.opensAt,
                {id: "story", presentation: {voting_portal_datetime_format: FORMAT}} as never,
                "en",
                device
            )
            // The DOM matcher collapses whitespace (Intl puts U+202F before AM/PM).
            await expect(
                nairobi.getByText(`On this device: ${onDevice.replace(/\s+/g, " ")}`)
            ).toBeVisible()
        }
    },
})

/** `tz-localization-voter`: overrides of a zone name and of the combined text. */
export const LocalizationOverrides = zonedStory({
    config: OVERSEAS,
    post: DUBAI,
    state: "open",
    testVoting: true,
    overrides: {
        en: {
            "votingPortal:timezones.name.Asia/Dubai": "Dubai time",
            "votingPortal:timezones.voterDateTimeZone": "{{dateTime}} ({{zoneName}})",
        },
    },
    play: async ({canvasElement}) => {
        const dubai = await card(canvasElement, DUBAI.title)
        await expect(dubai.getByText(/Apr 9, 2028, 12:00\sAM \(Dubai time\)/)).toBeVisible()
        await expect(
            dubai.getByText(/May 8, 2028, 7:00\sPM \(Philippine Standard Time\)/)
        ).toBeVisible()
    },
})

/** The second configuration: the central office, in the primary zone, shows one close line. */
export const AssociationCentral = zonedStory({
    config: ASSOCIATION,
    post: CENTRAL,
    state: "open",
    play: async ({canvasElement}) => {
        const central = await card(canvasElement, CENTRAL.title)
        await expect(central.getAllByText(/Central European (Summer )?Time/)).toHaveLength(2)
        await expect(
            canvasElement.querySelector(".election-close-date-local")
        ).not.toBeInTheDocument()
    },
})

/** The Canary Islands office: its opening in its zone, the close in Madrid's with its time below. */
export const AssociationCanaryPhone = zonedStory({
    config: ASSOCIATION,
    post: CANARY,
    state: "open",
    phone: true,
    play: async ({canvasElement}) => {
        const canary = await card(canvasElement, CANARY.title)
        await expect(
            canary.getByText(/Jun 2, 2028, 9:00\sAM Western European (Summer )?Time/)
        ).toBeVisible()
        await expect(
            canary.getByText(/Jun 9, 2028, 8:00\sPM Central European (Summer )?Time/)
        ).toBeVisible()
        await expect(
            canary.getByText(/Jun 9, 2028, 7:00\sPM Western European (Summer )?Time/)
        ).toBeVisible()
    },
})
