// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import ChannelIcon, {CHANNEL_ICON_NAMES, ChannelIconName, ChannelLabel} from "../ChannelIcon"
import VerticalBox from "../../VerticalBox/VerticalBox"

const LABELS: Record<ChannelIconName, string> = {
    EMAIL: "Email",
    SMS: "SMS",
    WHATSAPP: "WhatsApp",
    VIBER: "Viber",
    MESSENGER: "Facebook Messenger",
}

const ChannelIconsExample: React.FC<{fontSize: "small" | "medium" | "large"}> = ({fontSize}) => (
    <VerticalBox>
        {CHANNEL_ICON_NAMES.map((channel) => (
            <ChannelIcon
                key={channel}
                channel={channel}
                fontSize={fontSize}
                titleAccess={LABELS[channel]}
            />
        ))}
        {CHANNEL_ICON_NAMES.map((channel) => (
            <ChannelLabel key={`label-${channel}`} channel={channel} label={LABELS[channel]} />
        ))}
    </VerticalBox>
)

const meta: Meta<typeof ChannelIconsExample> = {
    title: "components/ChannelIcon",
    component: ChannelIconsExample,
    args: {fontSize: "medium"},
    argTypes: {fontSize: {control: "inline-radio", options: ["small", "medium", "large"]}},
    parameters: {backgrounds: {default: "white"}},
}

export default meta

type Story = StoryObj<typeof ChannelIconsExample>

export const Primary: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        for (const channel of CHANNEL_ICON_NAMES) {
            await expect(canvas.getByRole("img", {name: LABELS[channel]})).toBeVisible()
            await expect(
                canvas.getAllByText(LABELS[channel]).some((node) => node.tagName === "SPAN")
            ).toBe(true)
        }
    },
}

export const Large: Story = {args: {fontSize: "large"}}
