// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {IApplicationsStatus} from "@/types/applications"
import {ApprovalOutcomeChip} from "./ApprovalChips"

const meta = {
    title: "Admin/Approvals/ApprovalOutcomeChip",
    component: ApprovalOutcomeChip,
    args: {decision: IApplicationsStatus.ACCEPTED},
    argTypes: {
        decision: {control: "inline-radio", options: Object.values(IApplicationsStatus)},
    },
} satisfies Meta<typeof ApprovalOutcomeChip>
export default meta
type Story = StoryObj<typeof meta>

const chip = (canvasElement: HTMLElement, text: string) => within(canvasElement).getByText(text)

export const ApproveAutomatically: Story = {
    play: async ({canvasElement}) => {
        await expect(chip(canvasElement, "Approve automatically")).toBeVisible()
        await expect(chip(canvasElement, "Approve automatically")).toHaveAttribute(
            "data-status",
            "ACCEPTED"
        )
    },
}

export const SendToAPerson: Story = {
    args: {decision: IApplicationsStatus.PENDING},
    play: async ({canvasElement}) => {
        await expect(chip(canvasElement, "Send to a person")).toHaveAttribute(
            "data-status",
            "PENDING"
        )
    },
}

export const Reject: Story = {
    args: {decision: IApplicationsStatus.REJECTED},
    play: async ({canvasElement}) => {
        await expect(chip(canvasElement, "Reject")).toHaveAttribute("data-status", "REJECTED")
    },
}
