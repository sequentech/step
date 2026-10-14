// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ETaskExecutionStatus, i18n, browserTimeZone} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {taskLogs} from "./__stories__/WidgetFixture"
import {LogTable} from "./Widget"
import {formatZoned} from "@/lib/timezones/zonedFormat"

/** A time as the screen shows it outside an event's screens: labelled, in the viewer's zone. */
const shownTime = (value: string, seconds = true) =>
    formatZoned(value, browserTimeZone(), {t: i18n.t.bind(i18n), lang: i18n.language, seconds})

let boundary: ReturnType<typeof graphqlBoundary>

const DARK_RED = "rgb(139, 0, 0)"

const meta = {
    title: "Admin/Components/LogTable",
    component: LogTable,
    args: {logs: taskLogs, status: ETaskExecutionStatus.IN_PROGRESS},
    argTypes: {status: {control: "select", options: Object.values(ETaskExecutionStatus)}},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <LogTable {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof LogTable>
export default meta
type Story = StoryObj<typeof meta>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const rows = within(canvasElement).getAllByRole("row")
        expect(rows).toHaveLength(3)
        taskLogs.forEach(({created_date, log_text}, index) => {
            const [date, text] = within(rows[index]).getAllByRole("cell")
            // The time with its zone label, in the viewer's zone.
            expect(date).toHaveTextContent(shownTime(created_date))
            expect(text).toHaveTextContent(log_text)
            expect(getComputedStyle(text).color).not.toBe(DARK_RED)
        })
    },
}

export const FailedHighlightsLastLog: Story = {
    args: {status: ETaskExecutionStatus.FAILED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(getComputedStyle(canvas.getByText("Election event archive written")).color).toBe(
            DARK_RED
        )
        expect(getComputedStyle(canvas.getByText("Voters exported")).color).not.toBe(DARK_RED)
    },
}

export const Empty: Story = {
    args: {logs: []},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("table")).toBeInTheDocument()
        expect(within(canvasElement).queryByRole("row")).toBeNull()
    },
}
