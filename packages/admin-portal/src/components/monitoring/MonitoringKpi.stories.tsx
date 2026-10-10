// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {EColumnKind} from "./types"
import {MonitoringKpi} from "./MonitoringKpi"

const meta = {
    title: "Admin/Monitoring/MonitoringKpi",
    component: MonitoringKpi,
    args: {label: "Registered", value: 874624, kind: EColumnKind.INTEGER},
} satisfies Meta<typeof MonitoringKpi>
export default meta
type Story = StoryObj<typeof meta>

export const Count: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("874,624")).toBeVisible()
        await expect(canvas.getByText("Registered")).toBeVisible()
    },
}

export const Ratio: Story = {
    args: {label: "Voted of registered", value: 0.5323, kind: EColumnKind.NUMBER},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("53.2%")).toBeVisible()
    },
}

export const UndefinedRatio: Story = {
    args: {label: "Voted of pre-enrolled", value: null, kind: EColumnKind.NUMBER},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("—")).toBeVisible()
    },
}
