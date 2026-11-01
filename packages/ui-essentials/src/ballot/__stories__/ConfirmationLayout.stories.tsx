// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react-vite"
import {INITIAL_VIEWPORTS} from "storybook/viewport"

import {ConfirmationActions} from "../ConfirmationActions"
import {ConfirmationLayout} from "../ConfirmationLayout"
import {withCatalogue} from "./catalogue"

/**
 * The confirmation screen, design v42: the outcome and *Keep your receipt* on the left,
 * the receipt as a ticket on the right; one column on a phone. The portal adds the
 * status check, the Cast tag, the full Tracking ID and Get email updates when the event
 * has transparency; these stories draw the screen without them, as the Election
 * Architect's preview does, and with stand-ins where the portal's parts would go.
 */
const meta: Meta<typeof ConfirmationLayout> = {
    title: "ballot/ConfirmationLayout",
    component: ConfirmationLayout,
    parameters: {backgrounds: {default: "white"}, layout: "fullscreen"},
    decorators: [withCatalogue],
    args: {
        ballotId: "3f1c9a4e7b2d8065c1e94a7f30b6d2c58e7a1f094b3c6d2e5a8f71c0d9b4e263",
        ballotIdHref: "https://example.org/ballot-locator",
        qrValue: "https://example.org/ballot-locator",
        eventName: "Municipal elections 2026",
        electionName: "City council",
        castAt: "1 November 2026, 14:32",
        onBallotIdHelp: () => undefined,
        ballotIdCopyLabels: {copy: "Copy", copied: "Copied", error: "Copy it by hand"},
        actions: <ConfirmationActions onPrint={() => undefined} onFinish={() => undefined} />,
    },
}

export default meta

type Story = StoryObj<typeof ConfirmationLayout>

/** An event without transparency: the same ticket, with the whole Tracking ID. */
export const Primary: Story = {args: {}}

/** The short Tracking ID with stand-ins for the portal's tag and full Tracking ID. */
export const ShortTrackingId: Story = {
    args: {
        shortBallotId: "FTBE-MHRX",
        ballotIdCopyLabels: undefined,
        ballotIdTag: <span style={{fontSize: 13}}>Cast</span>,
        ballotIdDetails: (
            <div style={{fontFamily: "monospace", fontSize: 13, color: "#5B6474"}}>
                FTBE-MHRX-71J7-5ZBD-QPTD-YDMN-B47Q-MGAD-0ANJ-96AV-XZ2T-JG5R-KPGG
            </div>
        ),
        updates: (
            <div style={{fontSize: 15, color: "#4A5262"}}>
                The portal's optional Get email updates go here.
            </div>
        ),
    },
}

/** The cast time is not known yet: the line keeps its height. */
export const BeforeTheCastTime: Story = {args: {castAt: undefined}}

/** A caller that knows the cast email was sent says so in the row. */
export const WithCastEmail: Story = {
    args: {
        actions: (
            <ConfirmationActions
                onPrint={() => undefined}
                onFinish={() => undefined}
                castEmailSent
            />
        ),
    },
}

/** A ballot cast blank says so under the title. */
export const Blank: Story = {args: {isBlankBallot: true}}

/** Every contest acclaimed: no ticket, no ID, no Save receipt. */
export const Acclaimed: Story = {
    args: {
        isFullyAcclaimed: true,
        actions: <ConfirmationActions withPrint={false} onFinish={() => undefined} />,
    },
}

/** A phone: one column, with the ticket under the title. */
export const Phone: Story = {
    args: {},
    parameters: {viewport: {viewports: INITIAL_VIEWPORTS, defaultViewport: "iphone6"}},
}
