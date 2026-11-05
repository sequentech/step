// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {EventTimeZoneProvider, MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {EEditorBusy, MonitoringPreviewFooter} from "./MonitoringPreviewFooter"
import {EPreviewStatus} from "./yamlDraft"

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringPreviewFooter",
    component: MonitoringPreviewFooter,
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
        scopeLabel: "All authorized Posts",
        errors: 0,
        warnings: 0,
        previewStatus: EPreviewStatus.READY,
        renderMs: 42,
        revision: 7,
        savedAt: "2026-09-28T08:30:00Z",
        savedBy: "Ana Reyes",
        dirty: true,
        saveLabel: "Save widget",
        onCancel: fn(),
        onValidate: fn(),
        onSave: fn(),
    },
} satisfies Meta<typeof MonitoringPreviewFooter>
export default meta
type Story = StoryObj<typeof meta>

export const Valid: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const status = canvas.getByRole("status", {name: "Preview"})
        await expect(status).toHaveTextContent("Preview · All authorized Posts")
        await expect(status).toHaveTextContent("Valid")
        await expect(status).toHaveTextContent("no chart warnings")
        await expect(status).toHaveTextContent("rendered in 42 ms")
        await expect(status).toHaveTextContent("Revision 7")
        await expect(status).toHaveTextContent(/saved Sep 28, 2026, 4:30\s*PM PhST by Ana Reyes/)
        await userEvent.click(canvas.getByRole("button", {name: "Validate"}))
        await userEvent.click(canvas.getByRole("button", {name: "Save widget"}))
        await userEvent.click(canvas.getByRole("button", {name: "Cancel"}))
        expect(args.onValidate).toHaveBeenCalledTimes(1)
        expect(args.onSave).toHaveBeenCalledTimes(1)
        expect(args.onCancel).toHaveBeenCalledTimes(1)
    },
}

export const Invalid: Story = {
    args: {errors: 2, warnings: 1},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("status", {name: "Preview"})).toHaveTextContent("2 errors")
        await expect(canvas.getByRole("status", {name: "Preview"})).toHaveTextContent(
            "1 chart warning"
        )
        await expect(canvas.getByRole("button", {name: "Save widget"})).toBeDisabled()
    },
}

export const Saving: Story = {
    args: {busy: EEditorBusy.SAVING, previewStatus: EPreviewStatus.RENDERING},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("status", {name: "Preview"})).toHaveTextContent("Rendering…")
        await expect(canvas.getByRole("button", {name: "Save widget"})).toBeDisabled()
        await expect(canvas.getByRole("button", {name: "Cancel"})).toBeDisabled()
    },
}

export const NeverSaved: Story = {
    args: {revision: null, savedAt: null, dirty: false, onValidate: undefined},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("status", {name: "Preview"})).toHaveTextContent(
            "not saved yet"
        )
        expect(canvas.queryByRole("button", {name: "Validate"})).toBeNull()
    },
}
