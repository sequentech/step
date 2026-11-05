// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {ITallyExecutionStatus} from "@/types/ceremonies"
import {StatusChip} from "./StatusChip"

const meta = {
    title: "Admin/Components/StatusChip",
    component: StatusChip,
    args: {status: ITallyExecutionStatus.SUCCESS},
    argTypes: {status: {control: "select", options: Object.values(ITallyExecutionStatus)}},
} satisfies Meta<typeof StatusChip>
export default meta
type Story = StoryObj<typeof meta>

const lowContrast = {
    expectedFailure: {
        reason: "The chip's white label lacks contrast on the theme's success colour.",
        a11y: ["color-contrast"],
    },
}
const chip = (canvasElement: HTMLElement) =>
    canvasElement.querySelector<HTMLElement>(".MuiChip-root") as HTMLElement

export const Succeeded: Story = {
    parameters: lowContrast,
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("SUCCESS")).toBeVisible()
        // brandSuccess of the shared theme.
        expect(chip(canvasElement)).toHaveStyle({backgroundColor: "rgb(67, 227, 161)"})
    },
}

export const Failed: Story = {
    args: {status: ETaskExecutionStatus.FAILED},
    play: async ({canvasElement}) => {
        // Task executions share the chip with tally sessions.
        await expect(within(canvasElement).getByText("FAILED")).toBeVisible()
        expect(chip(canvasElement)).toHaveStyle({backgroundColor: "rgb(220, 38, 38)"})
    },
}

export const Cancelled: Story = {
    args: {status: ITallyExecutionStatus.CANCELLED},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("CANCELLED")).toBeVisible()
        // errorColor of the shared theme.
        expect(chip(canvasElement)).toHaveStyle({backgroundColor: "rgb(220, 38, 38)"})
    },
}

export const WithoutStatus: Story = {
    args: {status: ""},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("-")).toBeVisible()
    },
}
