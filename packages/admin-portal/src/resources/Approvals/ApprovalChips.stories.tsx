// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {IApplicationsStatus} from "@/types/applications"
import {ApprovalStatusChip} from "./ApprovalChips"

const meta = {
    title: "Admin/Approvals/ApprovalStatusChip",
    component: ApprovalStatusChip,
    args: {status: IApplicationsStatus.PENDING},
    argTypes: {
        status: {control: "inline-radio", options: Object.values(IApplicationsStatus)},
    },
} satisfies Meta<typeof ApprovalStatusChip>
export default meta
type Story = StoryObj<typeof meta>

const chip = (canvasElement: HTMLElement, text: string) => within(canvasElement).getByText(text)

export const NeedsReview: Story = {
    play: async ({canvasElement}) => {
        await expect(chip(canvasElement, "Needs review")).toBeVisible()
        await expect(chip(canvasElement, "Needs review")).toHaveAttribute("data-status", "PENDING")
    },
}

export const Approved: Story = {
    args: {status: IApplicationsStatus.ACCEPTED},
    play: async ({canvasElement}) => {
        await expect(chip(canvasElement, "Approved")).toHaveAttribute("data-status", "ACCEPTED")
    },
}

export const Rejected: Story = {
    args: {status: IApplicationsStatus.REJECTED},
    play: async ({canvasElement}) => {
        await expect(chip(canvasElement, "Rejected")).toHaveAttribute("data-status", "REJECTED")
    },
}

// The applications table stores the status in either case.
export const LowerCaseStatus: Story = {
    args: {status: "accepted"},
    play: async ({canvasElement}) => {
        await expect(chip(canvasElement, "Approved")).toBeVisible()
    },
}

export const WithoutAStatus: Story = {
    args: {status: null},
    play: async ({canvasElement}) => {
        await expect(chip(canvasElement, "Needs review")).toBeVisible()
    },
}
