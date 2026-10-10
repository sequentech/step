// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react-vite"
import BlankAnswer from "../BlankAnswer"
import {INITIAL_VIEWPORTS} from "storybook/viewport"

const meta: Meta<typeof BlankAnswer> = {
    title: "components/BlankAnswer",
    component: BlankAnswer,
    decorators: [
        (Story) => (
            <ul>
                <Story />
            </ul>
        ),
    ],
    parameters: {
        backgrounds: {
            default: "white",
        },
        viewport: {
            viewports: INITIAL_VIEWPORTS,
            defaultViewport: "iphone6",
        },
    },
}

export default meta

type Story = StoryObj<typeof BlankAnswer>

const parameters = {
    viewport: {
        disable: true,
    },
}

export const Primary: Story = {
    args: {},
    parameters,
}
