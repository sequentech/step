// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Provider} from "react-redux"
import {ApolloClient, ApolloLink, InMemoryCache} from "@apollo/client"
import {ApolloProvider} from "@apollo/client/react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within, userEvent, waitFor} from "storybook/test"
import {
    initCore,
    encryptBallotSelection,
    EConsolidatedReportPolicy,
    type IBallotStyle as BallotDefinition,
    type BallotSelection,
} from "@sequentech/ui-core"
import {electionFixture, IDS, FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {clearVoterSession, store} from "../../store/store"
import {setElection} from "../../store/elections/electionsSlice"
import {setBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {
    resetBallotSelection,
    setBallotSelection,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {setAuditableBallot} from "../../store/auditableBallots/auditableBallotsSlice"
import ReviewScreen from "../ReviewScreen"
import AuditScreen from "../AuditScreen"
import {ErrorPage} from "../ErrorPage"

const electionPath = `/tenant/${IDS.tenant}/event/${IDS.event}/election/${IDS.election}`
let client: ApolloClient
const meta = {
    title: "Voting/Ballot integrity",
    args: {mismatch: false, audit: false},
    parameters: {
        router: {
            path: "/tenant/:tenantId/event/:eventId/election/:electionId/review",
            initialEntries: [`${electionPath}/review?lang=en`],
            errorElement: <ErrorPage />,
        },
    },
    loaders: [
        async ({args}) => {
            await initCore()
            store.dispatch(clearVoterSession())
            client = new ApolloClient({
                cache: new InMemoryCache(),
                link: new ApolloLink(() => {
                    throw new Error("Unexpected GraphQL request from integrity story")
                }),
            })
            const ballot = electionFixture().ballot as unknown as BallotDefinition
            const style = {
                id: IDS.style,
                tenant_id: IDS.tenant,
                election_event_id: IDS.event,
                election_id: IDS.election,
                ballot_eml: ballot,
                created_at: FIXED_TIME,
                last_updated_at: FIXED_TIME,
            }
            const selection: BallotSelection = [
                {
                    contest_id: IDS.contest,
                    choices: [
                        {id: IDS.alice, selected: 0},
                        {id: IDS.bob, selected: -1},
                    ],
                    is_explicit_invalid: false,
                    is_decline_to_vote: false,
                    is_blank_ballot: false,
                    invalid_errors: [],
                    invalid_alerts: [],
                },
            ]
            const auditable = encryptBallotSelection(selection, ballot)
            if (args.mismatch) auditable.ballot_hash = "f".repeat(64)
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
                    },
                })
            )
            store.dispatch(setBallotStyle(style))
            store.dispatch(resetBallotSelection({ballotStyle: style, force: true}))
            store.dispatch(setBallotSelection({ballotStyle: style, ballotSelection: selection}))
            store.dispatch(
                setAuditableBallot({
                    electionId: IDS.election,
                    auditableBallot: auditable,
                    isBlankBallot: false,
                })
            )
        },
    ],
    render: ({audit}) => (
        <ApolloProvider client={client}>
            <Provider store={store}>
                <main>{audit ? <AuditScreen /> : <ReviewScreen />}</main>
            </Provider>
        </ApolloProvider>
    ),
} satisfies Meta<{mismatch: boolean; audit: boolean}>
export default meta
type Story = StoryObj<typeof meta>

export const ConsistentReview: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("button", {name: "Cast ballot"})).toBeEnabled()
        await expect(canvas.getByText("Alice Example", {exact: true})).toBeVisible()
        await expect(canvas.queryByRole("alert")).not.toBeInTheDocument()
    },
}

export const InconsistentReviewKeepsEdit: Story = {
    args: {mismatch: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("alert")).toHaveTextContent(
            "There was an error related to the ballot hashing process"
        )
        await expect(canvas.getByRole("alert")).toHaveTextContent("f".repeat(64))
        await expect(canvas.getByRole("button", {name: "Cast ballot"})).toBeDisabled()
        await userEvent.click(canvas.getByRole("link", {name: "Edit ballot"}))
        await waitFor(() =>
            expect(
                within(canvasElement.ownerDocument.body).getByRole("status", {
                    name: "Current location",
                })
            ).toHaveTextContent(`${electionPath}/vote?lang=en`)
        )
    },
}

export const InconsistentAuditRejectsDownload: Story = {
    args: {mismatch: true, audit: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("INCONSISTENT_HASH", {exact: true})).toBeVisible()
        await expect(canvas.queryByRole("button", {name: "Download"})).not.toBeInTheDocument()
    },
}
