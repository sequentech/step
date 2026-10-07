// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {formatZoned} from "@/lib/timezones/zonedFormat"
import {ZonedDateTimeText} from "./AdminDateTime"

/** "My time", explicit so the stories never depend on the machine's zone. */
const MY_ZONE = "America/New_York"
const AT = "2028-04-08T19:41:15Z"

interface Scenario {
    zone?: string
}

const meta = {
    title: "Admin/Components/ZonedDateTimeText",
    component: ZonedDateTimeText,
    args: {zone: "Asia/Dubai"},
    render: ({zone}: Scenario) => (
        <MyTimeZoneProvider zone={MY_ZONE}>
            <ZonedDateTimeText value={AT} zone={zone} />
        </MyTimeZoneProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const options = {t: i18n.t.bind(i18n), lang: i18n.language}

/** A zone the server gives (a signing panel's), labelled like every admin time. */
export const InAGivenZone: Story = {
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(formatZoned(AT, "Asia/Dubai", options))
        ).toBeVisible()
    },
}

/** Without a zone: mine. */
export const InMyZone: Story = {
    args: {zone: undefined},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(formatZoned(AT, MY_ZONE, options))
        ).toBeVisible()
    },
}
