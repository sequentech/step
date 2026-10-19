// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
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
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("All four checks passed")).toBeVisible()
        expect(canvas.getAllByText("Passed")).toHaveLength(4)
        expect(canvasElement.scrollWidth).toBeLessThanOrEqual(canvasElement.clientWidth + 1)
    },
}
export const Pending: Story = {
    args: {proof: {...fixture.proof, cast_entry: null}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Still being added to the log")).toBeVisible()
        expect(canvas.getAllByText("Passed")).toHaveLength(3)
        expect(canvas.getByText("Not in the log yet.")).toBeVisible()
    },
}
export const Tampered: Story = {
    args: {proof: {...fixture.proof, voting_channel: "KIOSK"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("A check failed")).toBeVisible()
        expect(canvas.getByText("Do not rely on this entry.")).toBeVisible()
        expect(canvas.queryByText("All four checks passed")).toBeNull()
    },
}
