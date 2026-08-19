// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"

import i18next from "i18next"
import {I18nextProvider} from "react-i18next"

import {BallotActions} from "../BallotActions"

/**
 * The portal's `votingScreen.*`, so this story shows words rather than keys.
 *
 * Story data, not product wording: the strings themselves live in
 * `voting-portal/src/translations/<lng>.ts`, which is the catalogue every real host of
 * this component supplies. A copy here would be the mistake `EA-F2-053` undid.
 */
const catalogue = i18next.createInstance()
void catalogue.init({
    lng: "en",
    fallbackLng: "en",
    resources: {
        en: {
            translation: {
                votingScreen: {
                    backButton: "Back",
                    clearButton: "Clear choices",
                    reviewButton: "Next",
                },
            },
        },
    },
    interpolation: {escapeValue: false},
})

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
    decorators: [
        (Story) => (
            <I18nextProvider i18n={catalogue}>
                <Story />
            </I18nextProvider>
        ),
    ],
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
