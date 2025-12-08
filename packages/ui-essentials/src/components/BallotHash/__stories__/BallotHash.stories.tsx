// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import BallotHash from "../BallotHash"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"

const meta: Meta<typeof BallotHash> = {
    title: "components/BallotHash",
    component: BallotHash,
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

type Story = StoryObj<typeof BallotHash>

export const Primary: Story = {
    args: {
        hash: "eee6fe54bc8a5f3fce2d2b8aa1909259ceaf7df3266302b7ce1a65ad85a53a92",
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
