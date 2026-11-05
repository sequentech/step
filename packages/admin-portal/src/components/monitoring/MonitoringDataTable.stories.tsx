// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {MonitoringDataTable} from "./MonitoringDataTable"
import {turnoutTable} from "./__stories__/MonitoringFixture"
import {EColumnKind, type MonitoringTable} from "./types"

/** A bucketed series of 1,050 rows: more than a page. */
const longTable: MonitoringTable = {
    columns: [
        {name: "bucket", kind: EColumnKind.TEXT},
        {name: "voted", kind: EColumnKind.INTEGER},
    ],
    rows: Array.from({length: 1050}, (_, index) => [`row-${index + 1}`, index * 3]),
}

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

/** More rows than a page: a page at a time, with a pager. */
export const ManyRows: Story = {
    args: {table: longTable, caption: "Turnout over time · data"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const table = canvas.getByRole("table", {name: "Turnout over time · data"})
        // The header and a page of 100 rows.
        expect(within(table).getAllByRole("row")).toHaveLength(101)
        await expect(canvas.getByText("1–100 of 1,050")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Last page"}))
        await expect(canvas.getByText("1,001–1,050 of 1,050")).toBeVisible()
        await expect(within(table).getByRole("row", {name: "row-1050 3,147"})).toBeVisible()
    },
}
