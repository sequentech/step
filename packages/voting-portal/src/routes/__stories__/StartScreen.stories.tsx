// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Provider} from "react-redux"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within, waitFor} from "storybook/test"
import {
    initCore,
    type IBallotStyle as BallotDefinition,
    EConsolidatedReportPolicy,
    ESecurityConfirmationPolicy,
    EDeclineToVotePolicy,
    EElectionEventContestEncryptionPolicy,
} from "@sequentech/ui-core"
import {electionFixture, IDS, FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import StartScreen from "../StartScreen"
import {clearVoterSession, store} from "../../store/store"
import {setElection} from "../../store/elections/electionsSlice"
import {setBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"

const electionPath = `/tenant/${IDS.tenant}/event/${IDS.event}/election/${IDS.election}`
const meta = {
    title: "Voting/Start screen",
    args: {demo: false, acclaimed: false},
    parameters: {
        router: {
            path: "/tenant/:tenantId/event/:eventId/election/:electionId/start",
            initialEntries: [`${electionPath}/start?lang=en`],
        },
    },
    loaders: [
        async ({args}) => {
            await initCore()
            sessionStorage.clear()
            store.dispatch(clearVoterSession())
            const ballot = electionFixture({demo: args.demo}).ballot as unknown as BallotDefinition
            ballot.election_event_presentation = {
                ...ballot.election_event_presentation!,
                contest_encryption_policy: EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS,
            }
            ballot.election_presentation = {
                ...ballot.election_presentation!,
                decline_to_vote_policy: EDeclineToVotePolicy.ENABLED,
            }
            ballot.contests[0].is_acclaimed = args.acclaimed
            store.dispatch(
                setElection({
                    id: IDS.election,
                    name: "Community Council",
                    tenant_id: IDS.tenant,
                    election_event_id: IDS.event,
                    image_document_id: "",
                    contests: ballot.contests,
                    presentation: {
                        consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
                        security_confirmation_policy: ESecurityConfirmationPolicy.NONE,
                        decline_to_vote_policy: EDeclineToVotePolicy.ENABLED,
                    },
                })
            )
            store.dispatch(
                setBallotStyle({
                    id: ballot.id,
                    tenant_id: IDS.tenant,
                    election_event_id: IDS.event,
                    election_id: IDS.election,
                    ballot_eml: ballot,
                    created_at: FIXED_TIME,
                    last_updated_at: FIXED_TIME,
                })
            )
        },
    ],
    render: () => (
        <Provider store={store}>
            <main>
                <StartScreen />
            </main>
        </Provider>
    ),
} satisfies Meta<{demo: boolean; acclaimed: boolean}>
export default meta
type Story = StoryObj<typeof meta>

export const DeclineConfirmation: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const body = within(canvasElement.ownerDocument.body)
        await userEvent.click(canvas.getByRole("button", {name: "Decline to Vote"}))
        let dialog = within(await body.findByRole("dialog", {name: "Confirm decline to vote"}))
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(body.queryByRole("dialog")).not.toBeInTheDocument())
        await expect(body.getByRole("status", {name: "Current location"})).toHaveTextContent(
            `${electionPath}/start?lang=en`
        )
        await userEvent.click(canvas.getByRole("button", {name: "Decline to Vote"}))
        dialog = within(await body.findByRole("dialog", {name: "Confirm decline to vote"}))
        await userEvent.click(dialog.getByRole("button", {name: "Decline to vote"}))
        await waitFor(() =>
            expect(body.getByRole("status", {name: "Current location"})).toHaveTextContent(
                `${electionPath}/review?lang=en`
            )
        )
    },
}

export const DemoAcknowledgement: Story = {
    args: {demo: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const body = within(canvasElement.ownerDocument.body)
        const dialog = within(await body.findByRole("dialog", {name: "Demo voting booth"}))
        await expect(dialog.getByText("Your vote will not be cast.")).toBeInTheDocument()
        await userEvent.click(
            dialog.getByRole("button", {name: "I understand that my vote will not be cast"})
        )
        await waitFor(() => expect(body.queryByRole("dialog")).not.toBeInTheDocument())
        await expect(canvas.getByRole("heading", {name: "How to vote"})).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Start Voting"})).toBeEnabled()
    },
}

export const AcclaimedElectionCannotDecline: Story = {
    args: {acclaimed: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.queryByRole("button", {name: "Decline to Vote"})
        ).not.toBeInTheDocument()
        await expect(canvas.getByRole("button", {name: "Start Voting"})).toBeEnabled()
    },
}
