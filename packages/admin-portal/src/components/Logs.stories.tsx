// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n, browserTimeZone} from "@sequentech/ui-core"
import {Logs} from "./Logs"
import {formatZoned} from "@/lib/timezones/zonedFormat"

/** A time as the screen shows it outside an event's screens: labelled, in the viewer's zone. */
const shownTime = (value: string, seconds = true) =>
    formatZoned(value, browserTimeZone(), {t: i18n.t.bind(i18n), lang: i18n.language, seconds})

const logs = [
    {created_date: "2026-01-15T12:00:00Z", log_text: "Keys ceremony created"},
    {created_date: "2026-01-15T12:05:30Z", log_text: "trustee1 generated a key fragment"},
]

const meta = {
    title: "Admin/Components/Logs",
    component: Logs,
    args: {logs},
    argTypes: {logs: {control: "object"}},
} satisfies Meta<typeof Logs>
export default meta
type Story = StoryObj<typeof meta>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("columnheader", {
                name: i18n.t("keysGeneration.ceremonyStep.logsHeader.date"),
            })
        ).toBeVisible()
        // The time with its zone label, in the viewer's zone.
        const row = canvas.getByRole("row", {name: /trustee1 generated a key fragment/})
        await expect(within(row).getByText(shownTime("2026-01-15T12:05:30Z"))).toBeVisible()
        expect(canvas.getAllByRole("row")).toHaveLength(3)
    },
}

export const Empty: Story = {
    args: {logs: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(i18n.t("keysGeneration.ceremonyStep.emptyLogs"))
        ).toBeVisible()
        expect(canvas.queryByRole("table")).not.toBeInTheDocument()
    },
}

export const CollapseHidesTheEntries: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const summary = canvas.getByRole("button", {
            name: i18n.t("keysGeneration.ceremonyStep.logsHeader.title"),
        })
        expect(summary).toHaveAttribute("aria-expanded", "true")
        await userEvent.click(summary)
        await waitFor(() => expect(summary).toHaveAttribute("aria-expanded", "false"))
        await waitFor(() => expect(canvas.getByText("Keys ceremony created")).not.toBeVisible())
    },
}
