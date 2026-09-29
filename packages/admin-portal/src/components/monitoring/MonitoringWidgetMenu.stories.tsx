// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {MonitoringWidgetMenu} from "./MonitoringWidgetMenu"

const meta = {
    title: "Admin/Monitoring/MonitoringWidgetMenu",
    component: MonitoringWidgetMenu,
    args: {widgetTitle: "Voter turnout", onViewData: fn(), onExport: fn()},
} satisfies Meta<typeof MonitoringWidgetMenu>
export default meta
type Story = StoryObj<typeof meta>

const open = async (canvasElement: HTMLElement) => {
    await userEvent.click(
        within(canvasElement).getByRole("button", {name: "Actions for Voter turnout"})
    )
    return within(await within(document.body).findByRole("menu"))
}

export const Viewer: Story = {
    play: async ({canvasElement, args}) => {
        const menu = await open(canvasElement)
        expect(menu.queryByRole("menuitem", {name: "Configure widget"})).toBeNull()
        expect(menu.queryByRole("menuitem", {name: "Duplicate"})).toBeNull()
        await userEvent.click(menu.getByRole("menuitem", {name: "Export CSV"}))
        await expect(args.onExport).toHaveBeenCalled()
    },
}

export const Configurer: Story = {
    args: {onConfigure: fn(), onDuplicate: fn()},
    play: async ({canvasElement, args}) => {
        const menu = await open(canvasElement)
        expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
            "Configure widget",
            "View data",
            "Export CSV",
            "Duplicate",
        ])
        await userEvent.click(menu.getByRole("menuitem", {name: "Configure widget"}))
        await expect(args.onConfigure).toHaveBeenCalled()
    },
}

export const NothingToShowYet: Story = {
    args: {onViewData: undefined, onExport: undefined},
    play: async ({canvasElement}) => {
        const menu = await open(canvasElement)
        await expect(menu.getByRole("menuitem", {name: "View data"})).toHaveAttribute(
            "aria-disabled",
            "true"
        )
        await expect(menu.getByRole("menuitem", {name: "Export CSV"})).toHaveAttribute(
            "aria-disabled",
            "true"
        )
    },
}
