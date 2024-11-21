// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import BlankAnswer from "../BlankAnswer"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"

const meta: Meta<typeof BlankAnswer> = {
    title: "components/BlankAnswer",
    component: BlankAnswer,
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
