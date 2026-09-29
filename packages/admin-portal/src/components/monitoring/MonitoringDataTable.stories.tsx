// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {MonitoringDataTable} from "./MonitoringDataTable"
import {turnoutTable} from "./__stories__/MonitoringFixture"

const meta = {
    title: "Admin/Monitoring/MonitoringDataTable",
    component: MonitoringDataTable,
    args: {table: turnoutTable, caption: "Turnout by group · data"},
} satisfies Meta<typeof MonitoringDataTable>
export default meta
type Story = StoryObj<typeof meta>

export const ExactValues: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const table = canvas.getByRole("table", {name: "Turnout by group · data"})
        await expect(within(table).getByRole("columnheader", {name: "pct"})).toBeVisible()
        // The renderer's ordering column is not data.
        expect(within(table).queryByRole("columnheader", {name: "position"})).toBeNull()
        await expect(
            within(table).getByRole("row", {name: /18–29 120,432 226,000 53.29%/})
        ).toBeVisible()
        // A ratio over zero is undefined, not 0%.
        await expect(within(table).getByRole("row", {name: /Unknown 1,204 0 —/})).toBeVisible()
    },
}

export const NoRows: Story = {
    args: {table: {...turnoutTable, rows: []}},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No rows")).toBeVisible()
    },
}
