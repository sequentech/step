// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {VoterMessagingSummary} from "./VoterMessagingSummary"

interface Scenario {
    attributes: Record<string, string[]>
}

const meta = {
    title: "Admin/User/VoterMessagingSummary",
    component: VoterMessagingSummary,
    args: {
        attributes: {
            "sequent.read-only.message-channel": ["WHATSAPP"],
            "sequent.read-only.whatsapp-number": ["+966501234567"],
            "sequent.read-only.verified-channels": ["WHATSAPP", "EMAIL"],
        },
    },
    argTypes: {attributes: {control: "object"}},
    render: ({attributes}) => <VoterMessagingSummary attributes={attributes} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const PrefersWhatsApp: Story = {
    parameters: {widgets: ["Row"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("+966501234567")).toBeVisible()
        expect(canvas.getByText("Not connected")).toBeVisible()
    },
}

export const ConnectedToMessenger: Story = {
    args: {
        attributes: {
            "sequent.read-only.message-channel": ["MESSENGER"],
            "sequent.read-only.messenger-id": ["7304419925518842"],
            "sequent.read-only.messenger-page": ["118204557331906"],
            "sequent.read-only.verified-channels": ["MESSENGER"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Connected")).toBeVisible()
    },
}

export const NoMessagingChannels: Story = {
    args: {attributes: {}},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No verified channels")).toBeVisible()
    },
}
