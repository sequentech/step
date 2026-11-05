// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {SimpleForm} from "react-admin"
import {ZonedDateTimeInput} from "./ZonedDateTimeInput"
import {MyTimeZoneProvider} from "./timeZoneService"

const meta = {
    title: "Admin/Timezones/ZonedDateTimeInput",
    component: ZonedDateTimeInput,
    args: {source: "scheduled", defaultZone: "America/Toronto", label: "Scheduled time"},
    decorators: [
        (Story: React.ComponentType) => (
            <AdminStoryProvider boundary={graphqlBoundary({})}>
                <MyTimeZoneProvider zone="UTC">
                    <SimpleForm
                        toolbar={false}
                        onSubmit={fn()}
                        record={{
                            id: "schedule",
                            scheduled: {
                                scheduled_date: "2028-03-12T07:30:00Z",
                                local: "2028-03-12T02:30",
                                timezone: "America/Toronto",
                            },
                        }}
                    >
                        <Story />
                    </SimpleForm>
                </MyTimeZoneProvider>
            </AdminStoryProvider>
        ),
    ],
}
export default meta
type Story = StoryObj<typeof meta>
export const GapExplainsResolution: Story = {
    parameters: {widgets: ["ZonedDateTimeField"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByLabelText("Scheduled time")).toHaveValue("2028-03-12T02:30")
        await expect(canvas.getByRole("alert")).toHaveTextContent(/does not exist in Toronto/)
        await expect(canvas.getByTestId("zoned-preview")).toHaveTextContent(/03:30/)
    },
}
