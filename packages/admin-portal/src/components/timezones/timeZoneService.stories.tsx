// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {MyTimeZoneProvider} from "./timeZoneService"
import {ZonedDateTime} from "./ZonedDateTime"

const meta = {
    title: "Admin/Timezones/MyTimeZoneProvider",
    component: MyTimeZoneProvider,
    args: {
        zone: "Asia/Manila",
        children: <ZonedDateTime instant="2028-05-08T11:00:00Z" zone="Asia/Manila" />,
    },
    decorators: [
        (Story: React.ComponentType) => (
            <AdminStoryProvider boundary={graphqlBoundary({})}>
                <Story />
            </AdminStoryProvider>
        ),
    ],
}
export default meta
type Story = StoryObj<typeof meta>
export const MatchingViewerZone: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/my time/)).toHaveTextContent(/19:00/)
        await expect(canvasElement.querySelectorAll("span span")).toHaveLength(1)
    },
}
export const DifferentViewerZone: Story = {
    args: {zone: "America/Toronto"},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText(/my time/)).toHaveTextContent(/07:00/)
    },
}
