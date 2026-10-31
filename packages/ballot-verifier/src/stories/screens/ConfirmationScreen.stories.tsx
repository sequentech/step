// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {initCore, type IContest, type IDecodedVoteContest} from "@sequentech/ui-core"
import {ConfirmationScreen} from "../../screens/ConfirmationScreen"
import type {IConfirmationBallot} from "../../services/BallotService"
import {TenantEventProvider} from "../../providers/TenantEventContext"

const hash = "ab".repeat(32)
const scope = {tenant_id: "tenant", election_event_id: "event", election_id: "election"}
const eventPath = "/tenant/tenant/event/event"
const importStep = new RegExp(`^${eventPath}/start$`)
const contest = (id: string, name: string, candidate: string): IContest => ({
    ...scope,
    id,
    name,
    min_votes: 0,
    max_votes: 1,
    winning_candidates_num: 1,
    is_encrypted: true,
    candidates: [{...scope, id: `${id}-candidate`, contest_id: id, name: candidate}],
})
const decoded = (id: string): IDecodedVoteContest => ({
    contest_id: id,
    is_explicit_invalid: false,
    is_decline_to_vote: false,
    is_blank_ballot: false,
    invalid_errors: [],
    invalid_alerts: [],
    choices: [{id: `${id}-candidate`, selected: 0}],
})
const confirmation = (): IConfirmationBallot => ({
    ballot_hash: hash,
    election_config: {
        ...scope,
        id: "style",
        area_id: "area",
        description: "Community Election",
        contests: [contest("council", "Council representative", "Alice Example")],
    },
    decoded_questions: [decoded("council")],
})
const meta = {
    title: "Screens/Verifier/Confirmation screen",
    component: ConfirmationScreen,
    args: {confirmationBallot: confirmation(), ballotId: hash},
    parameters: {
        backgrounds: {default: "white"},
        router: {
            path: "/tenant/:tenantId/event/:eventId/confirmation",
            initialEntries: [`${eventPath}/confirmation`],
        },
    },
    // App provides the route's tenant and event to every screen.
    decorators: [
        (Story) => (
            <TenantEventProvider tenantId="tenant" eventId="event">
                <Story />
            </TenantEventProvider>
        ),
    ],
    loaders: [
        async () => {
            await initCore()
            return {}
        },
    ],
} satisfies Meta<typeof ConfirmationScreen>
export default meta
type Story = StoryObj<typeof meta>

const candidateListFailure = {}
export const Primary: Story = {
    parameters: candidateListFailure,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("heading", {name: "Verify your ballot selections"})
        ).toBeVisible()
        await expect(canvas.getByText("Alice Example", {exact: true})).toBeVisible()
        await expect(
            canvas.queryByText("Does’t match the decoded ballot ID")
        ).not.toBeInTheDocument()
        await expect(canvas.getAllByText(hash)).toHaveLength(2)
    },
}
export const MismatchedId: Story = {
    args: {ballotId: "cd".repeat(32)},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Does’t match the decoded ballot ID")).toBeVisible()
        await expect(canvas.queryByText("Alice Example")).not.toBeInTheDocument()
        await expect(
            canvas.queryByRole("heading", {name: "Verify your ballot selections"})
        ).not.toBeInTheDocument()
    },
}
const multi = confirmation()
multi.election_config.contests.push(contest("school", "School representative", "Charlie Example"))
// Decoding order is deliberately reversed; display follows the published contest order.
multi.decoded_questions = [decoded("school"), decoded("council")]
export const MultipleContests: Story = {
    parameters: candidateListFailure,
    args: {confirmationBallot: multi},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const alice = canvas.getByText("Alice Example", {exact: true})
        const charlie = canvas.getByText("Charlie Example", {exact: true})
        await expect(alice).toBeVisible()
        await expect(charlie).toBeVisible()
        await expect(
            alice.compareDocumentPosition(charlie) & Node.DOCUMENT_POSITION_FOLLOWING
        ).toBeTruthy()
    },
}
const unknown = confirmation()
unknown.decoded_questions = [decoded("missing")]
export const MissingContest: Story = {
    args: {confirmationBallot: unknown},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Contest not found: missing")).toBeVisible()
    },
}
export const BackToImport: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("link", {name: "Back"}))
        await expect(canvas.getByLabelText("Current location")).toHaveTextContent(importStep)
    },
}
export const Loading: Story = {
    args: {confirmationBallot: null},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByLabelText("Current location")).toHaveTextContent(
            importStep
        )
    },
}
