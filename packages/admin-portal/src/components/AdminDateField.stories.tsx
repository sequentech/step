// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {i18n, type IElectionEventPresentation} from "@sequentech/ui-core"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EventTimeZoneProvider, MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {formatZoned} from "@/lib/timezones/zonedFormat"
import {AdminDateField} from "./AdminDateTime"

/** "My time", explicit so the stories never depend on the machine's zone. */
const MY_ZONE = "America/New_York"
const AT = "2028-04-08T19:41:15Z"

interface Scenario {
    primary: string
}

const presentation = (primary: string) =>
    ({timezones: {configured: [primary], primary}}) as IElectionEventPresentation

const meta = {
    title: "Admin/Components/AdminDateField",
    component: AdminDateField,
    args: {primary: "Asia/Manila"},
    argTypes: {primary: {control: "inline-radio", options: ["Asia/Manila", "Europe/Madrid"]}},
    render: ({primary}: Scenario) => (
        <MyTimeZoneProvider zone={MY_ZONE}>
            <EventTimeZoneProvider event={{id: EVENT_ID, presentation: presentation(primary)}}>
                <RecordContextProvider value={{id: 1, election_event_id: EVENT_ID, start_at: AT}}>
                    <AdminDateField source="start_at" seconds />
                </RecordContextProvider>
            </EventTimeZoneProvider>
        </MyTimeZoneProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const options = {t: i18n.t.bind(i18n), lang: i18n.language, seconds: true}

/** A list row's time, in its event's primary zone. */
export const ManilaPrimary: Story = {
    play: async ({canvasElement, args}) => {
        await expect(
            within(canvasElement).getByText(formatZoned(AT, args.primary, options))
        ).toBeVisible()
    },
}

export const MadridPrimary: Story = {
    args: {primary: "Europe/Madrid"},
    play: async ({canvasElement, args}) => {
        await expect(
            within(canvasElement).getByText(formatZoned(AT, args.primary, options))
        ).toBeVisible()
    },
}
