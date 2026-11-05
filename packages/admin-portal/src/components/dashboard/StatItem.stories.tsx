// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {Groups as GroupsIcon} from "@mui/icons-material"
import StatItem from "./StatItem"

const meta = {
    title: "Admin/Dashboard/StatItem",
    component: StatItem,
    args: {icon: <GroupsIcon data-testid="stat-icon" />, count: 1204, label: "Eligible voters"},
    argTypes: {icon: {table: {disable: true}}},
} satisfies Meta<typeof StatItem>
export default meta
type Story = StoryObj<typeof meta>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("1204")).toBeVisible()
        await expect(canvas.getByText("Eligible voters")).toBeVisible()
        await expect(canvas.getByTestId("stat-icon")).toBeVisible()
    },
}

export const ZeroCount: Story = {
    args: {count: 0, label: "Emails sent"},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("0")).toBeVisible()
    },
}

export const TextCount: Story = {
    args: {count: "12 / 40", label: "Areas voting"},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("12 / 40")).toBeVisible()
    },
}
