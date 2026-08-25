// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"

import {ReviewActions} from "../ReviewActions"
import {withCatalogue} from "./catalogue"

/**
 * The row under the review screen, as the voting portal draws it.
 *
 * Worth comparing `Primary` with `WithAudit`: the audit button is the unusual choice on
 * this screen and is coloured to say so, which is the property a client is checking when
 * they look at it.
 */
const meta: Meta<typeof ReviewActions> = {
    title: "ballot/ReviewActions",
    component: ReviewActions,
    parameters: {backgrounds: {default: "white"}},
    decorators: [withCatalogue],
    args: {onCast: () => undefined, onAudit: () => undefined},
}

export default meta

type Story = StoryObj<typeof ReviewActions>

/** Where the audit policy hides the button: two choices, back or cast. */
export const Primary: Story = {args: {}}

/** `EVotingPortalAuditButtonCfg.SHOW`: three, and one of them is a warning. */
export const WithAudit: Story = {args: {withAudit: true}}

/** Mid-cast: the chevron becomes a spinner and the button stops taking clicks. */
export const Casting: Story = {args: {withAudit: true, casting: true}}

/** In the Election Architect's preview: the row a voter meets, with nothing to cast to. */
export const Inert: Story = {
    args: {withAudit: true, onCast: undefined, onAudit: undefined, inert: true},
}

/** On a phone, where the three stack. */
export const Narrow: Story = {
    args: {withAudit: true},
    parameters: {
        viewport: {viewports: INITIAL_VIEWPORTS, defaultViewport: "iphone6"},
    },
}
