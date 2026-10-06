// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {EUnsignedScheduledClosePolicy, formatPlaceTime, i18n} from "@sequentech/ui-core"
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
    instantOf,
    lifecycleElections,
    lifecycleEvent,
    lifecycleSnapshotEntry,
} from "@/resources/ScheduledEvents/__stories__/LifecycleScheduleFixture"
import {ConfigurationAuthorizes} from "./ConfigurationAuthorizes"

interface Scenario {
    /** Whether an earlier approved configuration exists to compare with. */
    previous: boolean
}

const PUBLICATION_ID = "99999999-9999-4999-8999-999999999999"
const REQUEST_ID = "99999999-9999-4999-8999-999999999993"
const TEXT = {t: i18n.t, lang: "en", formatDateTime: adminDateTimeFormat("en")}

/** What the approval signs: the overseas schedule, a looser close and one opening signature. */
const subject = () => {
    const configuration = overseasConfiguration()
    const snapshot = lifecycleSnapshotEntry(configuration).snapshot
    return {
        ballot_publication_id: PUBLICATION_ID,
        schedule: snapshot.schedule,
        policies: {
            ...configuration.presentation.lifecycle_policies,
            unsigned_scheduled_close: EUnsignedScheduledClosePolicy.RUN_AS_SYSTEM,
        },
        open_voting: {required: true, signatures: 1},
        close_voting: {required: true, signatures: 2},
        // The channels each Post had when approved (ConfigurationSubject.post_channels).
        post_channels: {[configuration.elections[0].id]: ["ONLINE", "KIOSK"]},
    }
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Signing/ConfigurationAuthorizes",
    component: ConfigurationAuthorizes,
    args: {previous: true},
    beforeEach: ({args}) => {
        const configuration = overseasConfiguration()
        // The previous executed event-level approval signed a refused close and two opening signatures.
        const previous = {
            id: "99999999-9999-4999-8999-999999999992",
            code: "P3V-8QZ",
            scope_key: `${EVENT_ID}|approve-configuration|event`,
            executed_at: "2028-03-01T00:00:00Z",
            subject: {
                ...subject(),
                policies: {
                    ...configuration.presentation.lifecycle_policies,
                    unsigned_scheduled_close: EUnsignedScheduledClosePolicy.REFUSE,
                },
                open_voting: {required: true, signatures: 2},
            },
        }
        data = resourceBoundary({
            sequent_backend_area: [],
            sequent_backend_election: lifecycleElections(configuration),
            [EVENT_RESOURCE]: [lifecycleEvent(configuration)],
        })
        graphql = graphqlBoundary({
            GetConfigurationApprovals: () => ({
                data: {
                    current: {id: REQUEST_ID, scope_key: `${EVENT_ID}|approve-configuration|event`},
                    approvals: args.previous ? [previous] : [],
                },
            }),
        })
    },
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <MyTimeZoneProvider zone={MY_TIME_ZONE}>
                <ConfigurationAuthorizes
                    subject={subject()}
                    electionEventId={EVENT_ID}
                    requestId={REQUEST_ID}
                />
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const authorizes = (key: string, options?: Record<string, unknown>) =>
    i18n.t(`lifecycle.authorizes.${key}`, options)

/** The schedule in each Post's zone, and the rule and policy values signed. */
export const WhatItAuthorizes: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const dubai = overseasConfiguration().schedule[0]
        await expect(
            await canvas.findByText(
                authorizes("opens", {
                    time: formatPlaceTime(
                        instantOf(dubai.local, dubai.timezone),
                        dubai.timezone,
                        "Dubai PCG",
                        TEXT
                    ),
                })
            )
        ).toBeVisible()
        await expect(canvas.getByText(authorizes("rule.openNeeds", {count: 1}))).toBeVisible()
        await expect(
            canvas.getByText(
                authorizes("channelsOf", {
                    election: "Dubai PCG",
                    channels: `${i18n.t("common.channel.online")}, ${i18n.t("common.channel.kiosk")}`,
                })
            )
        ).toBeVisible()
        await expect(
            canvas.getByText(
                authorizes("unsignedClose", {
                    value: i18n.t("lifecycle.policies.close.runAsSystem.label"),
                })
            )
        ).toBeVisible()
    },
}

/** Against the previous approved configuration: each loosening is named. */
export const LoosensSincePrevious: Story = {
    play: async ({canvasElement}) => {
        const diff = within(await within(canvasElement).findByTestId("configuration-diff"))
        await expect(
            diff.getByText(
                authorizes("diff.loosens", {
                    setting: i18n.t("lifecycle.policies.close.title"),
                    before: i18n.t("lifecycle.policies.close.refuse.label"),
                    after: i18n.t("lifecycle.policies.close.runAsSystem.label"),
                })
            )
        ).toBeVisible()
        await expect(
            diff.getByText(
                authorizes("diff.loosens", {
                    setting: authorizes("rule.openSetting"),
                    before: authorizes("rule.signatures", {count: 2}),
                    after: authorizes("rule.signatures", {count: 1}),
                })
            )
        ).toBeVisible()
    },
}

/** The first approved configuration has nothing to compare with. */
export const FirstConfiguration: Story = {
    args: {previous: false},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(authorizes("firstConfiguration"))
        ).toBeVisible()
    },
}
