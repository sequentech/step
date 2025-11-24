// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import QRCode from "../QRCode"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"

const meta: Meta<typeof QRCode> = {
    title: "components/QRCode",
    component: QRCode,
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

type Story = StoryObj<typeof QRCode>

export const Primary: Story = {
    args: {
        value: "https://sequentech.io",
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
