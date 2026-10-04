// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {formatDateTimeZone, i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {MyTimeZoneProvider, adminDateTimeFormat} from "@/components/timezones/timeZoneService"
import {
    MY_TIME_ZONE,
    overseasConfiguration,
} from "@/components/timezones/__fixtures__/configurations"
import {
    EVENT_RESOURCE,
    lifecycleElections,
    lifecycleEvent,
    lifecycleSchedule,
} from "@/resources/ScheduledEvents/__stories__/LifecycleScheduleFixture"
import {FiredTransitions} from "./FiredTransitions"
import {
    FIRED_AT,
    closedAuthorized,
    closedNothingToChange,
    closedUnsigned,
    firedOutcome,
    openedAuthorized,
    openedRefused,
} from "./__stories__/FiredOutcomeFixture"

interface Scenario {
    /** What the event-wide close did at Dubai PCG, after its opening ran. */
    close: "authorized" | "unsigned" | "nothing" | "none"
    /** Whether Dubai PCG's scheduled opening was refused. */
    openingRefused: boolean
}

const CONFIGURATION = overseasConfiguration()
const DUBAI = CONFIGURATION.elections[0]
const TOKYO = CONFIGURATION.elections[1]
const TEXT = {t: i18n.t, lang: "en", formatDateTime: adminDateTimeFormat("en")}
const TIME = formatDateTimeZone(FIRED_AT, DUBAI.timezone!, TEXT)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Publish/FiredTransitions",
    component: FiredTransitions,
    args: {close: "authorized", openingRefused: false},
    beforeEach: ({args}) => {
        const schedule = lifecycleSchedule(CONFIGURATION)
        const opening = schedule[0]
        const close = schedule.find(({event_processor}) => event_processor === "END_VOTING_PERIOD")!
        opening.stopped_at = "2028-04-08T20:00:01Z"
        opening.annotations = {
            fired_outcome: firedOutcome(
                [
                    args.openingRefused
                        ? openedRefused(DUBAI.id, String(opening.id))
                        : openedAuthorized(DUBAI.id, String(opening.id)),
                ],
                "2028-04-08T20:00:00.456Z"
            ),
        }
        if (args.close !== "none") {
            close.stopped_at = FIRED_AT
            close.annotations = {
                // The event-wide close fired at every Post; this card reads Dubai's.
                fired_outcome: firedOutcome([
                    args.close === "authorized"
                        ? closedAuthorized(DUBAI.id, String(close.id))
                        : args.close === "nothing"
                          ? closedNothingToChange(DUBAI.id, String(close.id))
                          : closedUnsigned(DUBAI.id, String(close.id)),
                    closedUnsigned(TOKYO.id, String(close.id)),
                ]),
            }
        }
        data = resourceBoundary({
            sequent_backend_scheduled_event: schedule,
            sequent_backend_election: lifecycleElections(CONFIGURATION),
            [EVENT_RESOURCE]: [lifecycleEvent(CONFIGURATION)],
        })
        graphql = graphqlBoundary({})
    },
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <MyTimeZoneProvider zone={MY_TIME_ZONE}>
                <FiredTransitions electionEventId={EVENT_ID} electionId={DUBAI.id} />
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const publish = (key: string, options?: Record<string, unknown>) =>
    i18n.t(`lifecycle.publish.${key}`, options)

/** Closed on schedule, authorized by the signed configuration; its signers authorized it. */
export const ClosedAuthorized: Story = {
    play: async ({canvasElement}) => {
        const card = within(await within(canvasElement).findByTestId("closed-on-schedule"))
        await expect(
            card.getByRole("heading", {
                name: publish("closedAuthorized", {
                    time: TIME,
                    code: "K7Q-2M",
                    names: "Ana P. Reyes and Jose R. Dela Cruz",
                }),
            })
        ).toBeVisible()
        await expect(card.getByText(publish("authorizedBy"))).toBeVisible()
        await expect(within(canvasElement).getByTestId("opened-on-schedule")).toBeVisible()
    },
}

/** Closed without signatures: the neutral card names the close request it cancelled. */
export const ClosedUnsigned: Story = {
    args: {close: "unsigned"},
    play: async ({canvasElement}) => {
        const card = within(await within(canvasElement).findByTestId("closed-on-schedule"))
        await expect(
            card.getByRole("heading", {name: publish("closedUnsigned", {time: TIME})})
        ).toBeVisible()
        await expect(
            card.getByText(publish("cancelledRequest", {code: "R4D-9KX", n: 1, k: 2}))
        ).toBeVisible()
    },
}

/** The scheduled opening was refused: the card says so, with its "Why?". */
export const OpeningRefused: Story = {
    args: {close: "none", openingRefused: true},
    play: async ({canvasElement}) => {
        const card = within(await within(canvasElement).findByTestId("opened-on-schedule"))
        await expect(card.getByText(i18n.t("scheduledOutcome.chip.refused"))).toBeVisible()
        await userEvent.click(
            card.getByRole("button", {name: i18n.t("scheduledOutcome.why.button")})
        )
        const panel = within(await within(document.body).findByRole("dialog"))
        await waitFor(() =>
            expect(
                panel.getByText(i18n.t("scheduledOutcome.nextStep.publishAndApprove"))
            ).toBeVisible()
        )
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

/** The close found nothing open at the Post: the card says there was nothing to close. */
export const NothingToClose: Story = {
    args: {close: "nothing"},
    play: async ({canvasElement}) => {
        const card = within(await within(canvasElement).findByTestId("closed-on-schedule"))
        await expect(
            card.getByRole("heading", {name: publish("closedNothingToChange", {time: TIME})})
        ).toBeVisible()
    },
}
