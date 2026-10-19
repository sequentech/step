// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {atViewport} from "../../../test-support/storybook/viewport"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import initCore from "sequent-core"
import {BallotLogProof} from "@sequentech/ui-essentials"
import fixture from "./__stories__/ballotProof.json"

const meta = {
    title: "Voting/Ballot signatures",
    component: BallotLogProof,
    args: {trust: fixture.trusted, proof: fixture.proof},
    beforeEach: async () => {
        await initCore()
    },
} satisfies Meta<typeof BallotLogProof>
export default meta
type Story = StoryObj<typeof meta>

export const Verified: Story = {
    ...atViewport(1440),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("All four checks passed")).toBeVisible()
        expect(canvas.getAllByText("Passed")).toHaveLength(4)
        expect(canvasElement.scrollWidth).toBeLessThanOrEqual(canvasElement.clientWidth + 1)
    },
}
export const Pending: Story = {
    ...atViewport(1440),
    args: {proof: {...fixture.proof, cast_entry: null}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Still being added to the log")).toBeVisible()
        expect(canvas.getAllByText("Passed")).toHaveLength(3)
        expect(canvas.getByText("Not in the log yet.")).toBeVisible()
    },
}
export const Tampered: Story = {
    ...atViewport(1440),
    args: {proof: {...fixture.proof, voting_channel: "KIOSK"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("A check failed")).toBeVisible()
        expect(canvas.getByText("Do not rely on this entry.")).toBeVisible()
        expect(canvas.queryByText("All four checks passed")).toBeNull()
    },
}

export const VerifiedPhone: Story = {
    ...Verified,
    ...atViewport(390),
}

export const VerifiedTablet: Story = {
    ...Verified,
    ...atViewport(820),
}

export const PendingPhone: Story = {
    ...Pending,
    ...atViewport(390),
}

export const PendingTablet: Story = {
    ...Pending,
    ...atViewport(820),
}

export const TamperedPhone: Story = {
    ...Tampered,
    ...atViewport(390),
}

export const TamperedTablet: Story = {
    ...Tampered,
    ...atViewport(820),
}
