// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Provider} from "react-redux"
import {BallotSelectionAdapter} from "../../components/BallotSelectionAdapter"
import {ApolloClient, ApolloLink, InMemoryCache} from "@apollo/client"
import {ApolloProvider} from "@apollo/client/react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within, userEvent, waitFor} from "storybook/test"
import {
    initCore,
    encryptBallotSelection,
    EConsolidatedReportPolicy,
    EUnderVotePolicy,
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
import {ErrorPage} from "../ErrorPage"

interface Office {
    name: string
    seats: number
    candidates: Array<string>
    chosen: Array<string>
    policy: EUnderVotePolicy
}

const office = (
    name: string,
    seats: number,
    candidates: Array<string>,
    chosen: Array<string>,
    policy = EUnderVotePolicy.WARN_AND_CONFIRM_IN_REVIEW
): Office => ({name, seats, candidates, chosen, policy})

const PARTLY_FILLED: Array<Office> = [
    office("President", 1, ["Jordan Ellis", "Morgan Reyes"], ["Jordan Ellis"]),
    office("Vice President", 1, ["Cameron Lee", "Avery Stone"], []),
    office(
        "Secretary-Treasurer",
        1,
        ["Alex Parker", "Riley Quinn"],
        [],
        EUnderVotePolicy.WARN_ONLY_IN_REVIEW
    ),
    office(
        "Trustees",
        3,
        ["Rowan Scott", "Charlie Kim", "Dakota Reed", "Sam Ortiz"],
        ["Rowan Scott", "Charlie Kim"]
    ),
]

const FILLED: Array<Office> = [
    office("President", 1, ["Jordan Ellis", "Morgan Reyes"], ["Jordan Ellis"]),
    office(
        "Trustees",
        3,
        ["Rowan Scott", "Charlie Kim", "Dakota Reed", "Sam Ortiz"],
        ["Rowan Scott", "Charlie Kim", "Dakota Reed"]
    ),
]

const BLANK: Array<Office> = [
    office("President", 1, ["Jordan Ellis", "Morgan Reyes"], []),
    office("Trustees", 3, ["Rowan Scott", "Charlie Kim", "Dakota Reed", "Sam Ortiz"], []),
]

const uuid = (kind: number, index: number) =>
    `${kind}0000000-0000-4000-8000-${String(index).padStart(12, "0")}`

const electionPath = `/tenant/${IDS.tenant}/event/${IDS.event}/election/${IDS.election}`
let client: ApolloClient
const meta = {
    title: "Voting/Review unfilled positions",
    args: {offices: PARTLY_FILLED},
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
                    throw new Error("Unexpected GraphQL request from unfilled positions story")
                }),
            })
            const fixture = electionFixture({demo: true}).ballot
            const template = fixture.contests[0]
            const contests = args.offices.map((item, contestIndex) => {
                const contestId = uuid(6, contestIndex + 1)
                return {
                    ...template,
                    id: contestId,
                    name: item.name,
                    description: "",
                    min_votes: 0,
                    max_votes: item.seats,
                    winning_candidates_num: item.seats,
                    presentation: {
                        ...template.presentation,
                        under_vote_policy: item.policy,
                    },
                    candidates: item.candidates.map((name, candidateIndex) => ({
                        ...template.candidates[0],
                        id: uuid(7, contestIndex * 10 + candidateIndex + 1),
                        contest_id: contestId,
                        name,
                        presentation: {sort_order: candidateIndex},
                    })),
                }
            })
            const ballot = {...fixture, contests} as unknown as BallotDefinition
            const style = {
                id: IDS.style,
                tenant_id: IDS.tenant,
                election_event_id: IDS.event,
                election_id: IDS.election,
                ballot_eml: ballot,
                created_at: FIXED_TIME,
                last_updated_at: FIXED_TIME,
            }
            const selection: BallotSelection = contests.map((contest, index) => ({
                contest_id: contest.id,
                choices: contest.candidates.map((candidate) => ({
                    id: candidate.id,
                    selected: args.offices[index].chosen.includes(candidate.name) ? 0 : -1,
                })),
                is_explicit_invalid: false,
                is_decline_to_vote: false,
                is_blank_ballot: false,
                invalid_errors: [],
                invalid_alerts: [],
            }))
            store.dispatch(
                setElection({
                    id: IDS.election,
                    name: "Officer Election",
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
                    auditableBallot: encryptBallotSelection(selection, ballot),
                    isBlankBallot: false,
                })
            )
        },
    ],
    render: () => (
        <ApolloProvider client={client}>
            <Provider store={store}>
                <BallotSelectionAdapter>
                    <main>
                        <ReviewScreen />
                    </main>
                </BallotSelectionAdapter>
            </Provider>
        </ApolloProvider>
    ),
} satisfies Meta<{offices: Array<Office>}>
export default meta
type Story = StoryObj<typeof meta>

const castVotes = () => Object.values(store.getState().castVotes).flat().length
const dialogIn = (canvasElement: HTMLElement) =>
    within(canvasElement.ownerDocument.body).findByRole("dialog", {
        name: "Some selections are unfilled",
    })

export const NamesUnfilledPositionsBeforeCasting: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const body = within(canvasElement.ownerDocument.body)
        await userEvent.click(await canvas.findByRole("button", {name: "Cast ballot"}))

        const dialog = within(await dialogIn(canvasElement))
        const items = dialog.getAllByRole("listitem")
        await expect(items).toHaveLength(2)
        await expect(items[0]).toHaveTextContent("Vice President")
        await expect(items[0]).toHaveTextContent("0 of 1 selected · Nothing selected")
        await expect(items[1]).toHaveTextContent("Trustees")
        await expect(items[1]).toHaveTextContent("2 of 3 selected")
        await expect(items[1]).not.toHaveTextContent("Nothing selected")

        await userEvent.click(dialog.getByRole("button", {name: "Review selections"}))
        await waitFor(() => expect(body.queryByRole("dialog")).not.toBeInTheDocument())
        await expect(castVotes()).toBe(0)
        await expect(canvas.getByText("Jordan Ellis", {exact: true})).toBeVisible()
        await expect(canvas.getByRole("link", {name: "Edit ballot"})).toBeVisible()
    },
}

export const ContinuingCastsTheBallot: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Cast ballot"}))
        const dialog = within(await dialogIn(canvasElement))
        await userEvent.click(dialog.getByRole("button", {name: "Continue with these selections"}))
        await waitFor(() => expect(castVotes()).toBe(1))
    },
}

export const BlankBallotListsEveryContest: Story = {
    args: {offices: BLANK},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Cast ballot"}))
        const items = within(await dialogIn(canvasElement)).getAllByRole("listitem")
        await expect(items).toHaveLength(2)
        await expect(items[0]).toHaveTextContent("President")
        await expect(items[0]).toHaveTextContent("0 of 1 selected · Nothing selected")
        await expect(items[1]).toHaveTextContent("Trustees")
        await expect(items[1]).toHaveTextContent("0 of 3 selected · Nothing selected")
    },
}

export const FilledBallotCastsWithoutAsking: Story = {
    args: {offices: FILLED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const body = within(canvasElement.ownerDocument.body)
        await userEvent.click(await canvas.findByRole("button", {name: "Cast ballot"}))
        await waitFor(() => expect(castVotes()).toBe(1))
        await expect(body.queryByRole("dialog")).not.toBeInTheDocument()
    },
}
