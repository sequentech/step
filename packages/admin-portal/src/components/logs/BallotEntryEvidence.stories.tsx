// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import initCore from "sequent-core"
import ballotFixture from "../../../../voting-portal/src/components/__stories__/ballotProof.json"
import {BallotEntryEvidence} from "./BallotEntryEvidence"

const meta = {
    title: "Admin/Logs/BallotEntryEvidence",
    component: BallotEntryEvidence,
    beforeEach: async () => {
        await initCore()
    },
    args: {
        trust: ballotFixture.trusted,
        proof: ballotFixture.proof,
        selectedEntry: {id: "18443", statement_kind: "BallotReceived"},
        zone: "UTC",
        onRetry: fn(),
    },
} satisfies Meta<typeof BallotEntryEvidence>
export default meta
type Story = StoryObj<typeof meta>

export const Verified: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("All four checks passed")).toBeVisible()
        expect(canvas.getAllByRole("listitem")).toHaveLength(2)
        expect(canvas.queryByRole("button", {name: "Try again"})).toBeNull()
    },
}

export const PendingCast: Story = {
    args: {proof: {...ballotFixture.proof, cast_entry: null, voting_channel: ""}},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Still being added to the log")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Try again"}))
        expect(args.onRetry).toHaveBeenCalledOnce()
    },
}

export const InvalidSignature: Story = {
    args: {proof: {...ballotFixture.proof, cast_entry: "AAAA"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Do not rely on this entry.", {exact: false})).toBeVisible()
        expect(canvas.queryByText("All four checks passed")).toBeNull()
    },
}
