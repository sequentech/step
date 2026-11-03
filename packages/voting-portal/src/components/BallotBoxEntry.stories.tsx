// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {ApolloClient, ApolloLink, InMemoryCache, Observable} from "@apollo/client"
import {ApolloProvider} from "@apollo/client/react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import initCore from "sequent-core"
import {atViewport} from "../../../test-support/storybook/viewport"
import {BallotBoxEntry} from "./BallotBoxEntry"
import fixture from "./__stories__/ballotProof.json"

const client = new ApolloClient({
    cache: new InMemoryCache(),
    link: new ApolloLink(
        (operation) =>
            new Observable((observer) => {
                observer.next({
                    data:
                        operation.operationName === "BallotBoxEntry"
                            ? {ballot_box_entry: {payload: fixture.proof}}
                            : {ballot_box_keys: {payload: fixture.trusted}},
                })
                observer.complete()
            })
    ),
})
const meta = {
    title: "Voting/Ballot proof dialog",
    component: BallotBoxEntry,
    args: {
        electionEventId: fixture.trusted.election_event_id,
        electionId: fixture.trusted.election_id,
        ballotId: fixture.proof.ballot_id,
        onClose: fn(),
    },
    beforeEach: async () => {
        await initCore()
    },
    render: (args) => (
        <ApolloProvider client={client}>
            <BallotBoxEntry {...args} />
        </ApolloProvider>
    ),
} satisfies Meta<typeof BallotBoxEntry>
export default meta
type Story = StoryObj<typeof meta>

export const Verified: Story = {
    ...atViewport(1440),
    play: async ({args}) => {
        const dialog = await within(document.body).findByRole("dialog")
        const canvas = within(dialog)
        await canvas.findByText("All four checks passed")
        await waitFor(() => expect(canvas.getByText("All four checks passed")).toBeVisible())
        expect(canvas.queryByRole("button", {name: "Try again"})).toBeNull()
        const rect = dialog.getBoundingClientRect()
        expect(rect.width).toBeLessThanOrEqual(window.innerWidth)
        if (window.innerWidth < 600) expect(rect.width).toBe(window.innerWidth)
        else expect(rect.width).toBe(752)
        await userEvent.click(canvas.getByRole("button", {name: "Signed log entries (2)"}))
        await waitFor(() => expect(canvas.getByText(fixture.proof.received_entry)).toBeVisible())
        await userEvent.click(canvas.getByRole("button", {name: "Close"}))
        expect(args.onClose).toHaveBeenCalled()
    },
}
export const VerifiedTablet: Story = {...Verified, ...atViewport(820)}
export const VerifiedPhone: Story = {...Verified, ...atViewport(390)}
