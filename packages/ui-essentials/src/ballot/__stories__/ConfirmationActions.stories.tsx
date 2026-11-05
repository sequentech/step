// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react-vite"

import {ConfirmationActions} from "../ConfirmationActions"
import {withCatalogue} from "./catalogue"

/** The row under *Keep your receipt*: an outlined Save receipt (PDF), and Finish at the end. */
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

/** A caller that knows the cast email was sent says so between the two. */
export const WithCastEmail: Story = {args: {castEmailSent: true}}
