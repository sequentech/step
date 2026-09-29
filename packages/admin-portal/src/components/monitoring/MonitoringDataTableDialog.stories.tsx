// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {MonitoringDataTableDialog} from "./MonitoringDataTableDialog"
import {turnoutTable} from "./__stories__/MonitoringFixture"

const meta = {
    title: "Admin/Monitoring/MonitoringDataTableDialog",
    component: MonitoringDataTableDialog,
    args: {
        open: true,
        onClose: fn(),
        title: "Turnout by group",
        scope: "North · All authorized Posts · All countries",
        table: turnoutTable,
        notices: ["Unregistered attempts are counted at event scope only."],
    },
} satisfies Meta<typeof MonitoringDataTableDialog>
export default meta
type Story = StoryObj<typeof meta>

/** The open dialog, once its fade-in has finished. */
const dialog = async () => {
    const element = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

export const ViewData: Story = {
    play: async ({args}) => {
        const body = await dialog()
        await expect(body.getByText("Turnout by group · data")).toBeVisible()
        await expect(body.getByText("North · All authorized Posts · All countries")).toBeVisible()
        await expect(
            body.getByText("Unregistered attempts are counted at event scope only.")
        ).toBeVisible()
        await expect(body.getByRole("table", {name: "Turnout by group · data"})).toBeVisible()
        await userEvent.click(body.getByRole("button", {name: "Close"}))
        await expect(args.onClose).toHaveBeenCalled()
    },
}
