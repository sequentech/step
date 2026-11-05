// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {SimpleForm} from "react-admin"
import {LogRangeDateTimeInput} from "./LogRangeDateTimeInput"

const meta = {
    title: "Admin/Logs/LogRangeDateTimeInput",
    component: LogRangeDateTimeInput,
    args: {source: "created_from", label: "Created from"},
    decorators: [
        (Story: React.ComponentType) => (
            <AdminStoryProvider boundary={graphqlBoundary({})}>
                <SimpleForm
                    toolbar={false}
                    onSubmit={fn()}
                    record={{
                        id: "range",
                        created_from: "2028-11-05T01:30",
                        time_zone: "America/Toronto",
                    }}
                >
                    <Story />
                </SimpleForm>
            </AdminStoryProvider>
        ),
    ],
}
export default meta
type Story = StoryObj<typeof meta>
export const OverlapKeepsWallTime: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByLabelText("Created from")).toHaveValue("2028-11-05T01:30")
        await expect(canvas.getByText(/happens twice in Toronto/)).toBeVisible()
    },
}
