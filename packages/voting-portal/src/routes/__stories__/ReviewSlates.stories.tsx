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
    decodeAuditableBallot,
    decodeAuditableMultiBallot,
    encryptBallotSelection,
    encryptMultiBallotSelection,
    EConsolidatedReportPolicy,
    EElectionEventContestEncryptionPolicy,
    EMobileCandidateLists,
    SLATES_ANNOTATION,
    type IBallotStyle as BallotDefinition,
    type BallotSelection,
    type ISlatesConfig,
} from "@sequentech/ui-core"
import {electionFixture, IDS, FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {clearVoterSession, store} from "../../store/store"
import {setElection} from "../../store/elections/electionsSlice"
import {setBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {
    resetBallotSelection,
    setBallotSelection,
    setBallotSelectionVoteChoice,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {setAuditableBallot} from "../../store/auditableBallots/auditableBallotsSlice"
import ReviewScreen from "../ReviewScreen"
import {ErrorPage} from "../ErrorPage"

const electionPath = `/tenant/${IDS.tenant}/event/${IDS.event}/election/${IDS.election}`
const SLATES: ISlatesConfig = {
    version: 1,
    mobile_candidate_lists: EMobileCandidateLists.COLLAPSED,
    slates: [
        {id: "forward", name: {en: "Forward Together"}, members: {[IDS.contest]: [IDS.alice]}},
    ],
}
let client: ApolloClient

const meta = {
    title: "Voting/Review with slates",
    args: {selected: IDS.alice, changedAfterEncrypting: false, multiContest: false},
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
                    throw new Error("Unexpected GraphQL request from review story")
                }),
            })
            const fixture = electionFixture().ballot
            const ballot = {
                ...fixture,
                election_annotations: {[SLATES_ANNOTATION]: JSON.stringify(SLATES)},
                election_event_presentation: {
                    ...fixture.election_event_presentation,
                    contest_encryption_policy: args.multiContest
                        ? EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS
                        : EElectionEventContestEncryptionPolicy.SINGLE_CONTEST,
                },
            } as unknown as BallotDefinition
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
                    choices: [IDS.alice, IDS.bob].map((id) => ({
                        id,
                        selected: id === args.selected ? 0 : -1,
                    })),
                    is_explicit_invalid: false,
                    is_decline_to_vote: false,
                    is_blank_ballot: false,
                    invalid_errors: [],
                    invalid_alerts: [],
                },
            ]
            const encrypt = () => {
                if (args.multiContest) {
                    const multiBallot = encryptMultiBallotSelection(selection, ballot)
                    return [multiBallot, decodeAuditableMultiBallot(multiBallot)] as const
                }
                const singleBallot = encryptBallotSelection(selection, ballot)
                return [singleBallot, decodeAuditableBallot(singleBallot)] as const
            }
            const [auditable, encryptedSelection] = encrypt()
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
            store.dispatch(
                setAuditableBallot({
                    electionId: IDS.election,
                    auditableBallot: auditable,
                    isBlankBallot: false,
                })
            )
            // As the voting screen does: the store holds what the ballot decodes to.
            store.dispatch(
                setBallotSelection({
                    ballotStyle: style,
                    ballotSelection: encryptedSelection ?? selection,
                })
            )
            if (args.changedAfterEncrypting) {
                store.dispatch(
                    setBallotSelectionVoteChoice({
                        ballotStyle: style,
                        contestId: IDS.contest,
                        voteChoice: {id: IDS.bob, selected: 0},
                    })
                )
            }
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
} satisfies Meta<{selected: string; changedAfterEncrypting: boolean; multiContest: boolean}>
export default meta
type Story = StoryObj<typeof meta>

const currentLocation = (canvasElement: HTMLElement) =>
    within(canvasElement.ownerDocument.body).getByRole("status", {name: "Current location"})

export const SlateSelected: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const summary = within(await canvas.findByRole("region", {name: "Your selections"}))
        await expect(summary.getByText("Candidates selected: 1 of 1")).toBeVisible()
        await expect(summary.getByText("Forward Together: All 1 selected")).toBeVisible()
        await expect(canvas.getByText("1 of 1 selected")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Cast ballot"})).toBeEnabled()
        await expect(currentLocation(canvasElement)).toHaveTextContent(
            `${electionPath}/review?lang=en`
        )
    },
}

export const MultiContestBallotSelected: Story = {
    args: {multiContest: true},
    play: SlateSelected.play,
}

export const MultiContestBallotChangedAfterEncrypting: Story = {
    args: {multiContest: true, changedAfterEncrypting: true},
    play: async (context) => ChangedAfterEncryptingReturnsToVoting.play?.(context),
}

export const IndependentSelected: Story = {
    args: {selected: IDS.bob},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const summary = within(await canvas.findByRole("region", {name: "Your selections"}))
        await expect(summary.getByText("Independent candidates selected: 1")).toBeVisible()
        await expect(summary.queryByText(/Forward Together/)).not.toBeInTheDocument()
    },
}

export const EditReturnsToTheContest: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            await canvas.findByRole("link", {name: "Edit Council representative"})
        )
        await waitFor(() =>
            expect(currentLocation(canvasElement)).toHaveTextContent(`${electionPath}/vote?lang=en`)
        )
    },
}

export const ChangedAfterEncryptingReturnsToVoting: Story = {
    args: {changedAfterEncrypting: true},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(currentLocation(canvasElement)).toHaveTextContent(`${electionPath}/vote?lang=en`)
        )
        await expect(
            within(canvasElement).queryByRole("button", {name: "Cast ballot"})
        ).not.toBeInTheDocument()
    },
}
