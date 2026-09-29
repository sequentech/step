// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {IMonitoringEditorApi} from "./api"
import {MonitoringResetToPresetDialog} from "./MonitoringResetToPresetDialog"
import {fakeEditorApi} from "./storyFixtures"

let api: IMonitoringEditorApi
type TApiOverrides = () => Partial<IMonitoringEditorApi>

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringResetToPresetDialog",
    component: MonitoringResetToPresetDialog,
    args: {open: true, api: fakeEditorApi(), onClose: fn(), onReset: fn()},
    beforeEach: ({parameters}) => {
        api = fakeEditorApi((parameters.api as TApiOverrides | undefined)?.() ?? {})
    },
    render: (args) => <MonitoringResetToPresetDialog {...args} api={api} />,
} satisfies Meta<typeof MonitoringResetToPresetDialog>
export default meta
type Story = StoryObj<typeof meta>

const dialog = async (canvasElement: HTMLElement) => {
    const found = await within(canvasElement.ownerDocument.body).findByRole("dialog", {
        name: "Reset to preset",
    })
    await waitFor(() => expect(found).toBeVisible())
    return within(found)
}

export const ChooseAndConfirm: Story = {
    play: async ({canvasElement, args}) => {
        const view = await dialog(canvasElement)
        await userEvent.click(await view.findByRole("radio", {name: "Campus elections · v1"}))
        await expect(view.getByText(/replaced by the preset's/)).toBeVisible()
        await userEvent.click(view.getByRole("button", {name: "Reset"}))
        await expect(await view.findByText("The event now uses Campus elections.")).toBeVisible()
        expect(api.resetToPreset).toHaveBeenCalledWith("campus")
        expect(args.onReset).toHaveBeenCalledWith(4, expect.objectContaining({id: "campus"}))
    },
}

export const Failed: Story = {
    parameters: {
        api: (() => ({
            resetToPreset: fn(async () => {
                throw new Error("locked down")
            }),
        })) satisfies TApiOverrides,
    },
    play: async ({canvasElement}) => {
        const view = await dialog(canvasElement)
        await userEvent.click(await view.findByRole("button", {name: "Reset"}))
        await expect(await view.findByText("The reset failed: locked down")).toBeVisible()
        await expect(view.getByRole("button", {name: "Reset"})).toBeEnabled()
    },
}

export const NoPresets: Story = {
    parameters: {api: (() => ({listPresets: fn(async () => [])})) satisfies TApiOverrides},
    play: async ({canvasElement}) => {
        const view = await dialog(canvasElement)
        await expect(await view.findByText("No presets are available.")).toBeVisible()
        await expect(view.getByRole("button", {name: "Reset"})).toBeDisabled()
    },
}
