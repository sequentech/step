// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import initCore from "sequent-core"
import ballotFixture from "../../../../voting-portal/src/components/__stories__/ballotProof.json"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {BallotEntryProof} from "./BallotEntryProof"

interface Scenario {
    state: "verified" | "unavailable"
}
let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Logs/BallotEntryProof",
    component: BallotEntryProof,
    args: {state: "verified"},
    beforeEach: async ({args}) => {
        await initCore()
        boundary = graphqlBoundary(
            {
                BallotBoxEntry: () =>
                    args.state === "unavailable"
                        ? {errors: [new GraphQLError("Synthetic evidence unavailable")]}
                        : {data: {ballot_box_entry: {payload: ballotFixture.proof}}},
                BallotBoxKeys: () => ({data: {ballot_box_keys: {payload: ballotFixture.trusted}}}),
            },
            {schema: true}
        )
        await boundary.ready
    },
    render: () => (
        <AdminStoryProvider boundary={boundary}>
            <BallotEntryProof
                electionEventId={ballotFixture.trusted.election_event_id}
                electionId={ballotFixture.trusted.election_id}
                ballotId={ballotFixture.proof.ballot_id}
                pseudonym={ballotFixture.proof.pseudonym}
                selectedEntry={{id: "18443", statement_kind: "BallotReceived"}}
                zone="UTC"
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Verified: Story = {
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("All four checks passed")).toBeVisible()
        expect(boundary.calls.map(({name}) => name).sort()).toEqual([
            "BallotBoxEntry",
            "BallotBoxKeys",
        ])
        expect(
            boundary.calls.every(({headers}) => headers["x-hasura-role"] === IPermissions.LOGS_READ)
        ).toBe(true)
    },
}

export const Unavailable: Story = {
    args: {state: "unavailable"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "The signed entries or published keys could not be loaded. Try again."
            )
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Try again"}))
        await waitFor(() =>
            expect(boundary.calls.filter(({name}) => name === "BallotBoxEntry")).toHaveLength(2)
        )
        expect(canvas.queryByText("All four checks passed")).toBeNull()
    },
}
