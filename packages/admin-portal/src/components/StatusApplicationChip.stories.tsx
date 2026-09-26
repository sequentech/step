// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {theme} from "@sequentech/ui-essentials"
import {IApplicationsStatus} from "@/types/applications"
import {StatusApplicationChip} from "./StatusApplicationChip"

/** The CSS `rgb()` form a browser reports for a `#rrggbb` palette colour. */
const rgb = (hex: string) =>
    `rgb(${[1, 3, 5].map((start) => parseInt(hex.slice(start, start + 2), 16)).join(", ")})`

const chipBackground = (canvasElement: HTMLElement, label: string) => {
    const text = within(canvasElement).getByText(label)
    return getComputedStyle(text.parentElement as HTMLElement).backgroundColor
}

const meta = {
    title: "Admin/Components/StatusApplicationChip",
    component: StatusApplicationChip,
    args: {status: IApplicationsStatus.PENDING},
    argTypes: {
        status: {control: "select", options: [...Object.values(IApplicationsStatus), ""]},
    },
} satisfies Meta<typeof StatusApplicationChip>
export default meta
type Story = StoryObj<typeof meta>

export const Pending: Story = {
    parameters: {
        expectedFailure: {
            reason: "The white label on the light warning background of a pending chip lacks contrast.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("PENDING")).toBeVisible()
        expect(chipBackground(canvasElement, "PENDING")).toBe(rgb(theme.palette.warning.light))
    },
}

export const Accepted: Story = {
    args: {status: IApplicationsStatus.ACCEPTED},
    parameters: {
        expectedFailure: {
            reason: "The white label on the light green background of an accepted chip lacks contrast.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("ACCEPTED")).toBeVisible()
        expect(chipBackground(canvasElement, "ACCEPTED")).toBe(rgb(theme.palette.brandSuccess))
    },
}

export const Rejected: Story = {
    args: {status: IApplicationsStatus.REJECTED},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("REJECTED")).toBeVisible()
        expect(chipBackground(canvasElement, "REJECTED")).toBe(rgb(theme.palette.errorColor))
    },
}

export const WithoutStatus: Story = {
    args: {status: ""},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("-")).toBeVisible()
    },
}
