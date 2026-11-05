// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {ZonedDateTime} from "./ZonedDateTime"
import {MyTimeZoneProvider} from "./timeZoneService"

const meta = {
    title: "Admin/Timezones/ZonedDateTime",
    component: ZonedDateTime,
    args: {instant: "2028-05-08T11:00:00Z", zone: "Asia/Manila"},
    decorators: [
        (Story: React.ComponentType) => (
            <AdminStoryProvider boundary={graphqlBoundary({})}>
                <MyTimeZoneProvider zone="America/Toronto">
                    <Story />
                </MyTimeZoneProvider>
            </AdminStoryProvider>
        ),
    ],
}
export default meta
type Story = StoryObj<typeof meta>
export const PlaceAndViewer: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/19:00/)).toBeVisible()
        await expect(canvas.getByText(/my time/)).toHaveTextContent(/07:00/)
    },
}
export const MissingInstant: Story = {
    args: {instant: null},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("-")).toBeVisible()
    },
}
