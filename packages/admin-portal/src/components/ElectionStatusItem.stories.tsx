// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ITallyElectionStatus} from "@/types/ceremonies"
import {ElectionStatusItem} from "./ElectionStatusItem"

const meta = {
    title: "Admin/Components/ElectionStatusItem",
    component: ElectionStatusItem,
    args: {name: ITallyElectionStatus.SUCCESS},
    argTypes: {name: {control: "select", options: Object.values(ITallyElectionStatus)}},
} satisfies Meta<typeof ElectionStatusItem>
export default meta
type Story = StoryObj<typeof meta>

const lowContrast = {
    expectedFailure: {
        reason: "The white status label lacks contrast on the theme's success and info colours.",
        a11y: ["color-contrast"],
    },
}

/** The coloured badge around the status label. */
const badge = (canvasElement: HTMLElement, label: string) =>
    within(canvasElement).getByText(label).parentElement as HTMLElement

export const Completed: Story = {
    parameters: lowContrast,
    play: async ({canvasElement}) => {
        // brandSuccess of the shared theme.
        await expect(badge(canvasElement, "SUCCESS")).toHaveStyle({
            backgroundColor: "rgb(67, 227, 161)",
        })
    },
}

export const Mixing: Story = {
    args: {name: ITallyElectionStatus.MIXING},
    parameters: lowContrast,
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("MIXING")).toBeVisible()
    },
}

export const Failed: Story = {
    args: {name: ITallyElectionStatus.ERROR},
    play: async ({canvasElement}) => {
        // errorColor of the shared theme.
        await expect(badge(canvasElement, "ERROR")).toHaveStyle({
            backgroundColor: "rgb(220, 38, 38)",
        })
    },
}

export const NotReported: Story = {
    args: {name: undefined},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("-")).toBeVisible()
    },
}
