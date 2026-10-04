// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {expect, within} from "storybook/test"
import {ELogTimeZonePolicy, i18n, type IElectionEventPresentation} from "@sequentech/ui-core"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import {EventTimeZoneProvider, MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {formatZoned} from "@/lib/timezones/zonedFormat"
import {AdminDateTime} from "./AdminDateTime"

/** "My time", explicit so the stories never depend on the machine's zone. */
const MY_ZONE = "America/New_York"
const AT = "2028-04-08T19:41:15Z"

/** Two configurations: a Manila primary and a Madrid one. */
const PRESENTATIONS: Record<string, IElectionEventPresentation> = {
    manila: {
        timezones: {
            configured: ["Asia/Manila", "Asia/Dubai"],
            primary: "Asia/Manila",
            logs: ELogTimeZonePolicy.PRIMARY,
        },
    } as IElectionEventPresentation,
    madrid: {
        timezones: {
            configured: ["Europe/Madrid", "Atlantic/Canary"],
            primary: "Europe/Madrid",
            logs: ELogTimeZonePolicy.ELECTION,
        },
    } as IElectionEventPresentation,
}

interface Scenario {
    configuration?: keyof typeof PRESENTATIONS
    seconds?: boolean
}

const options = (seconds = false) => ({t: i18n.t.bind(i18n), lang: i18n.language, seconds})
const primaryOf = (configuration: keyof typeof PRESENTATIONS) =>
    PRESENTATIONS[configuration].timezones!.primary

const meta = {
    title: "Admin/Components/AdminDateTime",
    component: AdminDateTime,
    args: {configuration: "manila", seconds: false},
    argTypes: {configuration: {control: "inline-radio", options: Object.keys(PRESENTATIONS)}},
    render: ({configuration, seconds}: Scenario) => (
        <MyTimeZoneProvider zone={MY_ZONE}>
            <EventTimeZoneProvider
                event={
                    configuration
                        ? {id: EVENT_ID, presentation: PRESENTATIONS[configuration]}
                        : undefined
                }
            >
                <AdminDateTime value={AT} event={EVENT_ID} seconds={seconds} />
            </EventTimeZoneProvider>
        </MyTimeZoneProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** One value with a label, in the event's primary zone. */
export const InTheEventsPrimary: Story = {
    play: async ({canvasElement}) => {
        const text = formatZoned(AT, primaryOf("manila"), options())
        await expect(within(canvasElement).getByText(text)).toBeVisible()
    },
}

export const MadridPrimaryWithSeconds: Story = {
    args: {configuration: "madrid", seconds: true},
    play: async ({canvasElement}) => {
        const text = formatZoned(AT, primaryOf("madrid"), options(true))
        await expect(within(canvasElement).getByText(text)).toBeVisible()
    },
}

/** Outside an event's screens: the viewer's zone, still labelled. */
export const OutsideAnEvent: Story = {
    args: {configuration: undefined},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(formatZoned(AT, MY_ZONE, options()))
        ).toBeVisible()
    },
}
