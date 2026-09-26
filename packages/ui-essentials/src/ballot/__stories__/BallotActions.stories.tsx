// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"

import {BallotActions} from "../BallotActions"
import {withCatalogue} from "./catalogue"

/**
 * The buttons under a ballot, as the voting portal draws them.
 *
 * Worth looking at on a phone: there are two Clear buttons in the tree and the
 * viewport decides which one is shown, so `Narrow` below is not the same arrangement
 * with smaller buttons — it is a different one.
 */
const meta: Meta<typeof BallotActions> = {
    title: "ballot/BallotActions",
    component: BallotActions,
    parameters: {
        backgrounds: {
            default: "white",
        },
    },
    decorators: [withCatalogue],
}

export default meta

type Story = StoryObj<typeof BallotActions>

export const Primary: Story = {
    args: {},
}

/** A contest is over-voted, so there is nowhere to go yet. */
export const NextRefused: Story = {
    args: {disableNext: true},
}

/** In the Election Architect's preview: the real row, and nothing it can do. */
export const Inert: Story = {
    args: {inert: true},
}

/** On a phone, Clear moves above Back and Next and goes full width. */
export const Narrow: Story = {
    args: {},
    parameters: {
        viewport: {
            viewports: INITIAL_VIEWPORTS,
            defaultViewport: "iphone6",
        },
    },
}
