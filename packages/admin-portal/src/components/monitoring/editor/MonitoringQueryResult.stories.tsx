// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {MonitoringQueryResult} from "./MonitoringQueryResult"
import {RENDERED} from "./storyFixtures"

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringQueryResult",
    component: MonitoringQueryResult,
    args: {table: RENDERED.table},
} satisfies Meta<typeof MonitoringQueryResult>
export default meta
type Story = StoryObj<typeof meta>

export const FirstRows: Story = {
    args: {
        table: {
            columns: [
                {name: "post", kind: "text"},
                {name: "voted", kind: "integer"},
            ],
            rows: Array.from({length: 8}, (_, index) => [
                `Post ${index + 1}`,
                index === 2 ? null : index,
            ]),
        },
    },
    play: async ({canvasElement}) => {
        const table = within(canvasElement).getByRole("table")
        // A header row and the first five rows only.
        expect(within(table).getAllByRole("row")).toHaveLength(6)
        await expect(within(table).getByText("—")).toBeVisible()
        expect(within(table).queryByText("Post 6")).toBeNull()
    },
}

export const NoRows: Story = {
    args: {table: null},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText(/no rows/i)).toBeVisible()
    },
}
