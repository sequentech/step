// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {useZonedFormat} from "@/hooks/useZonedFormat"
import {formatMine, formatZoned} from "@/lib/timezones/zonedFormat"
import {LogTime} from "./LogTime"

/** "My time", explicit so the stories never depend on the machine's zone. */
const MY_ZONE = "America/New_York"
/** 2028-04-08T22:00:03Z. */
const SECONDS = Date.UTC(2028, 3, 8, 22, 0, 3) / 1000

interface Scenario {
    zone: string
}

function Fixture({zone}: Scenario) {
    const format = useZonedFormat(zone, {seconds: true})
    return <LogTime seconds={SECONDS} zone={zone} format={format} />
}

const options = {t: i18n.t.bind(i18n), lang: i18n.language, seconds: true}

const meta = {
    title: "Admin/Logs/LogTime",
    component: LogTime,
    args: {zone: "Asia/Manila"},
    argTypes: {
        zone: {control: "inline-radio", options: ["Asia/Manila", "Atlantic/Canary", MY_ZONE]},
    },
    render: (args: Scenario) => (
        <MyTimeZoneProvider zone={MY_ZONE}>
            <Fixture {...args} />
        </MyTimeZoneProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** The row's log zone, then my time below. */
export const LogZoneAndMyTime: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const at = SECONDS * 1000
        await expect(canvas.getByText(formatZoned(at, args.zone, options))).toBeVisible()
        await expect(canvas.getByText(formatMine(at, args.zone, options, MY_ZONE)!)).toBeVisible()
    },
}

/** One line when the log zone is mine. */
export const OneLineInMyZone: Story = {
    args: {zone: MY_ZONE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(formatZoned(SECONDS * 1000, MY_ZONE, options))).toBeVisible()
        expect(canvas.queryByText(/my time/)).toBeNull()
    },
}
