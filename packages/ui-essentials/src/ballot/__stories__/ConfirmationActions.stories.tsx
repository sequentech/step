// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"

import {ConfirmationActions} from "../ConfirmationActions"
import {withCatalogue} from "./catalogue"

/** The row under the confirmation screen: a secondary Print, and Finish. */
const meta: Meta<typeof ConfirmationActions> = {
    title: "ballot/ConfirmationActions",
    component: ConfirmationActions,
    parameters: {backgrounds: {default: "white"}},
    decorators: [withCatalogue],
    args: {onPrint: () => undefined, onFinish: () => undefined},
}

export default meta

type Story = StoryObj<typeof ConfirmationActions>

export const Primary: Story = {args: {}}

/** The receipt is being made. */
export const Printing: Story = {args: {printing: true}}

/** In a preview: there is no receipt, because there is no cast vote. */
export const NothingToPrint: Story = {args: {onPrint: undefined}}
