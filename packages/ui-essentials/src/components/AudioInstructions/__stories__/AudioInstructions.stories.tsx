// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {StoryFn, Meta} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {EAudioInstructionsPolicy} from "@sequentech/ui-core"
import AudioInstructions from "../AudioInstructions"

export default {
    title: "components/AudioInstructions",
    component: AudioInstructions,
    parameters: {
        backgrounds: {
            default: "light",
        },
    },
} as Meta<typeof AudioInstructions>

const Template: StoryFn<typeof AudioInstructions> = (args) => <AudioInstructions {...args} />

const text =
    "This is your ballot. Use the Tab key to move between options and the Space bar to select or clear one."

export const Synthesized = Template.bind({})
Synthesized.args = {
    policy: EAudioInstructionsPolicy.RECORDED_OR_SYNTHESIZED,
    text,
    language: "en",
}

export const Transcript = Template.bind({})
Transcript.args = {...Synthesized.args}
Transcript.play = async ({canvasElement}) => {
    const canvas = within(canvasElement)
    await userEvent.click(canvas.getByRole("button", {name: "Read the instructions"}))
    await expect(canvas.getByText(text)).toBeVisible()
    await expect(canvas.getByRole("button", {name: "Hide the instructions"})).toHaveAttribute(
        "aria-expanded",
        "true"
    )
}
