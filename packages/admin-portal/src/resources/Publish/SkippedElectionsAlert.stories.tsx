// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// VOTE-FREEZE: the elections an event-wide Start or Resume left as they
// were, each with the server's reason, until the administrator dismisses
// the list.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    ESkippedElectionReason,
    type ISkippedElection,
    SkippedElectionsAlert,
} from "./SkippedElectionsAlert"

interface Scenario {
    skipped: ISkippedElection[]
    onClose: () => void
}

const MADRID_POST: ISkippedElection = {
    election_id: "33333333-3333-4333-8333-333333333331",
    election_name: "Madrid Post",
    reason: ESkippedElectionReason.BALLOT_BOX_SEAL_POLICY,
}

const meta = {
    title: "Admin/Publish/SkippedElectionsAlert",
    component: SkippedElectionsAlert,
    args: {skipped: [MADRID_POST], onClose: fn()},
    argTypes: {skipped: {control: "object"}},
    render: ({skipped, onClose}) => <SkippedElectionsAlert skipped={skipped} onClose={onClose} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** Seal at close keeps a Post whose voting has closed closed; the alert says why. */
export const ClosedBySealAtClose: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Some elections stay closed")).toBeVisible()
        await expect(
            canvas.getByText(
                "Madrid Post stays closed: its voting has closed and Seal at close makes closing final."
            )
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Dismiss"}))
        await expect(args.onClose).toHaveBeenCalledOnce()
    },
}

/** An election without a name is named by its id; an unknown reason is shown as is. */
export const UnnamedElectionAndUnknownReason: Story = {
    args: {
        skipped: [
            MADRID_POST,
            {election_id: "33333333-3333-4333-8333-333333333332", reason: "future-reason"},
        ],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getAllByRole("listitem")).toHaveLength(2)
        await expect(
            canvas.getByText(
                "33333333-3333-4333-8333-333333333332 was left as it was (future-reason)."
            )
        ).toBeVisible()
    },
}

/** Nothing was skipped: no alert. */
export const NothingSkipped: Story = {
    args: {skipped: []},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByRole("alert")).toBeNull()
    },
}
