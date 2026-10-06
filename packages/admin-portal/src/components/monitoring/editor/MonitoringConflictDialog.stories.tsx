// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {EventTimeZoneProvider, MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {MonitoringConflictDialog} from "./MonitoringConflictDialog"
import {WIDGET_YAML} from "./storyFixtures"

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringConflictDialog",
    component: MonitoringConflictDialog,
    decorators: [
        (Story) => (
            <MyTimeZoneProvider zone="America/New_York">
                <EventTimeZoneProvider
                    event={{
                        id: "monitoring-event",
                        presentation: {
                            timezones: {
                                configured: ["Asia/Manila"],
                                primary: "Asia/Manila",
                                logs: "primary",
                            },
                        },
                    }}
                >
                    <Story />
                </EventTimeZoneProvider>
            </MyTimeZoneProvider>
        ),
    ],
    args: {
        open: true,
        mine: WIDGET_YAML.replace("title: Turnout by group", "title: Turnout by voter group"),
        theirs: WIDGET_YAML.replace("height: 240", "height: 300"),
        currentRevision: 9,
        author: {id: "u-luis", name: "Luis Santos"},
        time: "2026-09-29T09:15:00Z",
        onReload: fn(),
        onKeepEditing: fn(),
        copyText: fn(async () => undefined),
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <MonitoringConflictDialog {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof MonitoringConflictDialog>
export default meta
type Story = StoryObj<typeof meta>

/** The dialog once its fade-in is over, so visibility checks mean something. */
const dialog = async (canvasElement: HTMLElement) => {
    const found = await within(canvasElement.ownerDocument.body).findByRole("dialog", {
        name: "Someone saved first",
    })
    await waitFor(() => expect(found).toBeVisible())
    return found
}

export const Conflict: Story = {
    play: async ({canvasElement, args}) => {
        const view = within(await dialog(canvasElement))
        await expect(view.getByText(/Luis Santos saved revision 9/)).toBeVisible()
        await expect(view.getByText(/Luis Santos saved revision 9/)).toHaveTextContent(
            /Sep 29, 2026, 5:15\s*PM PhST/
        )
        const mine = await view.findByRole("region", {name: "My changes"})
        await expect(within(mine).getByText(/Turnout by voter group/)).toBeVisible()
        const saved = view.getByRole("region", {name: "Saved revision"})
        await expect(within(saved).getByText(/height: 300/)).toBeVisible()
        // The YAML as written, not as a JSON list of its lines.
        await expect(saved.textContent).toMatch(/^height: 300$/m)
        await expect(mine.textContent).not.toMatch(/^\s*"/m)

        await userEvent.click(view.getByRole("button", {name: "Copy my YAML"}))
        expect(args.copyText).toHaveBeenCalledWith(args.mine)
        await expect(await view.findByText("Your YAML is on the clipboard.")).toBeVisible()

        await userEvent.click(view.getByRole("button", {name: "Reload"}))
        expect(args.onReload).toHaveBeenCalledTimes(1)
        await userEvent.click(view.getByRole("button", {name: "Keep editing"}))
        expect(args.onKeepEditing).toHaveBeenCalledTimes(1)
    },
}

export const ClipboardUnavailable: Story = {
    args: {
        author: null,
        time: null,
        copyText: fn(async () => {
            throw new Error("denied")
        }),
    },
    play: async ({canvasElement}) => {
        const view = within(await dialog(canvasElement))
        await expect(view.getByText("Revision 9 was saved while you were editing.")).toBeVisible()
        await view.findByRole("region", {name: "My changes"})
        await userEvent.click(view.getByRole("button", {name: "Copy my YAML"}))
        await expect(await view.findByText(/clipboard is unavailable/)).toBeVisible()
    },
}

export const LoadingSavedRevision: Story = {
    args: {theirs: undefined},
    play: async ({canvasElement}) => {
        const view = within(await dialog(canvasElement))
        await expect(view.getByRole("button", {name: "Reload"})).toBeDisabled()
    },
}
