// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {Provider} from "react-redux"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within, waitFor, fn} from "storybook/test"
import {
    initCore,
    type IBallotStyle as BallotDefinition,
    CandidatesOrder,
    EInvalidVotePolicy,
    EBlankVotePolicy,
    EConsolidatedReportPolicy,
    EBlankBallotsPolicy,
    EElectionEventContestEncryptionPolicy,
} from "@sequentech/ui-core"
import {electionFixture, IDS, FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import VotingScreen, {action} from "../VotingScreen"
import {clearVoterSession, store} from "../../store/store"
import {setElection, type IElectionExtended} from "../../store/elections/electionsSlice"
import {setBallotStyle, type IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {resetBallotSelection} from "../../store/ballotSelections/ballotSelectionsSlice"

type Scenario =
    | "blocked"
    | "pagination"
    | "warning"
    | "back-start"
    | "back-chooser"
    | "blank"
    | "wasm-error"
    | "exhausted"
import {decode_auditable_multi_ballot_js, type IDecodedVoteContest} from "sequent-core"
import {AuthContext} from "../../providers/AuthContextProvider"
import {addCastVotes, CastVoteStatus} from "../../store/castVotes/castVotesSlice"
import {ErrorPage} from "../ErrorPage"

const logout = fn()
function Screen() {
    const auth = useContext(AuthContext)
    return (
        <AuthContext.Provider value={{...auth, logout}}>
            <VotingScreen />
        </AuthContext.Provider>
    )
}

const electionPath = `/tenant/${IDS.tenant}/event/${IDS.event}/election/${IDS.election}`

function prepare(scenario: Scenario) {
    const ballot = electionFixture().ballot as unknown as BallotDefinition
    const contest = ballot.contests[0]
    contest.presentation = {
        candidates_order: CandidatesOrder.CUSTOM,
        sort_order: 0,
        invalid_vote_policy: EInvalidVotePolicy.NOT_ALLOWED,
        blank_vote_policy: EBlankVotePolicy.NOT_ALLOWED,
    }
    if (scenario === "warning") {
        contest.min_votes = 0
        contest.presentation.invalid_vote_policy = EInvalidVotePolicy.ALLOWED
    }
    if (scenario === "pagination" || scenario === "blank") {
        contest.presentation.pagination_policy = "first"
        ballot.contests.push({
            ...contest,
            id: "second-contest",
            name: "School representative",
            presentation: {...contest.presentation, pagination_policy: "second", sort_order: 1},
            candidates: contest.candidates.map((candidate, index) => ({
                ...candidate,
                id: `school-candidate-${index}`,
                contest_id: "second-contest",
                name: index === 0 ? "Charlie Example" : "Dana Example",
            })),
        })
    }
    if (scenario === "blank") {
        ballot.election_presentation = {
            ...ballot.election_presentation!,
            blank_ballots_policy: EBlankBallotsPolicy.ENABLED,
        }
        ballot.election_event_presentation = {
            ...ballot.election_event_presentation!,
            contest_encryption_policy: EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS,
        }
        for (const item of ballot.contests) {
            item.min_votes = 0
            item.presentation = {
                ...item.presentation,
                pagination_policy: "same",
                blank_vote_policy: EBlankVotePolicy.WARN,
                invalid_vote_policy: EInvalidVotePolicy.ALLOWED,
            }
        }
    }
    if (scenario === "wasm-error")
        Object.assign(ballot.election_event_presentation!, {contest_encryption_policy: "unsupported-encryption-policy"})
    const ballotStyle: IBallotStyle = {
        id: ballot.id,
        tenant_id: IDS.tenant,
        election_event_id: IDS.event,
        election_id: IDS.election,
        ballot_eml: ballot,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
    const election: IElectionExtended = {
        id: IDS.election,
        name: "Community Council",
        tenant_id: IDS.tenant,
        election_event_id: IDS.event,
        image_document_id: "",
        contests: ballot.contests,
        num_allowed_revotes: scenario === "exhausted" ? 1 : 0,
        presentation: {
            blank_ballots_policy:
                scenario === "blank" ? EBlankBallotsPolicy.ENABLED : EBlankBallotsPolicy.DISABLED,
            consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
            voting_screen_back_policy:
                scenario === "back-start" ? "start-screen" : "election-selection-screen",
        },
    }
    logout.mockClear()
    store.dispatch(clearVoterSession())
    store.dispatch(setElection(election))
    store.dispatch(setBallotStyle(ballotStyle))
    store.dispatch(resetBallotSelection({ballotStyle, force: true}))
    if (scenario === "exhausted")
        store.dispatch(
            addCastVotes([
                {
                    id: "used-vote",
                    tenant_id: IDS.tenant,
                    election_event_id: IDS.event,
                    election_id: IDS.election,
                    status: CastVoteStatus.VALID,
                },
            ])
        )
}

const meta = {
    title: "Voting/Ballot screen",
    args: {scenario: "blocked" as Scenario},
    parameters: {
        router: {
            parentPath: "/tenant/:tenantId/event/:eventId/election/:electionId",
            path: "vote",
            initialEntries: [`${electionPath}/vote?lang=en`],
            action,
            errorElement: <ErrorPage />,
        },
    },
    loaders: [
        async ({args}) => {
            await initCore()
            prepare(args.scenario)
        },
    ],
    render: () => (
        <Provider store={store}>
            <main>
                <Screen />
            </main>
        </Provider>
    ),
} satisfies Meta<{scenario: Scenario}>
export default meta
type Story = StoryObj<typeof meta>

export const SinglePageBlockingDialog: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const body = within(canvasElement.ownerDocument.body)
        const next = await canvas.findByRole("button", {name: "Next"})
        await expect(next).toBeEnabled()
        await userEvent.click(next)
        const dialog = within(await body.findByRole("dialog"))
        await waitFor(() =>
            expect(dialog.getByRole("button", {name: "Review selection"})).toBeVisible()
        )
        await expect(dialog.queryByRole("button", {name: /continue/i})).not.toBeInTheDocument()
        await userEvent.click(dialog.getByRole("button", {name: "Review selection"}))
        await waitFor(() => expect(body.queryByRole("dialog")).not.toBeInTheDocument())
        await expect(body.getByRole("status", {name: "Current location"})).toHaveTextContent(
            `${electionPath}/vote?lang=en`
        )
    },
}

export const PaginationFocusAndClear: Story = {
    args: {scenario: "pagination"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const next = await canvas.findByRole("button", {name: "Next"})
        await expect(next).toBeDisabled()
        await userEvent.click(canvas.getByRole("checkbox", {name: /Alice Example/}))
        await expect(next).toBeEnabled()
        await userEvent.click(next)
        await waitFor(() => expect(canvas.getByText("Step 2 of 2", {exact: true})).toHaveFocus())
        await expect(next).toBeDisabled()
        const charlie = canvas.getByRole("checkbox", {name: /Charlie Example/})
        await userEvent.click(charlie)
        await expect(next).toBeEnabled()
        await userEvent.click(canvas.getByRole("button", {name: "Clear choices"}))
        await expect(charlie).not.toBeChecked()
        await expect(next).toBeDisabled()
        await userEvent.click(canvas.getByRole("button", {name: "Back"}))
        await waitFor(() => expect(canvas.getByText("Step 1 of 2", {exact: true})).toHaveFocus())
        await expect(canvas.getByRole("checkbox", {name: /Alice Example/})).toBeChecked()
    },
}

export const OvervoteWarningCancelAndContinue: Story = {
    args: {scenario: "warning"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const body = within(canvasElement.ownerDocument.body)
        await userEvent.click(await canvas.findByRole("checkbox", {name: /Alice Example/}))
        await userEvent.click(canvas.getByRole("checkbox", {name: /Bob Example/}))
        await userEvent.click(canvas.getByRole("button", {name: "Next"}))
        const dialog = within(await body.findByRole("dialog"))
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(body.queryByRole("dialog")).not.toBeInTheDocument())
        await expect(canvas.getByRole("checkbox", {name: /Alice Example/})).toBeChecked()
        await expect(canvas.getByRole("checkbox", {name: /Bob Example/})).toBeChecked()
        await userEvent.click(canvas.getByRole("button", {name: "Next"}))
        await userEvent.click(
            within(await body.findByRole("dialog")).getByRole("button", {name: "Continue"})
        )
        await waitFor(() =>
            expect(body.getByRole("status", {name: "Current location"})).toHaveTextContent(
                `${electionPath}/review?lang=en`
            )
        )
    },
}

export const BackToStart: Story = {
    args: {scenario: "back-start"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Back"}))
        await expect(
            within(canvasElement.ownerDocument.body).getByRole("status", {name: "Current location"})
        ).toHaveTextContent(`${electionPath}/start?lang=en`)
    },
}

export const BackToChooser: Story = {
    args: {scenario: "back-chooser"},
    parameters: {
        router: {
            parentPath: "/",
            path: "/tenant/:tenantId/event/:eventId/election/:electionId/vote",
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Back"}))
        await expect(
            within(canvasElement.ownerDocument.body).getByRole("status", {name: "Current location"})
        ).toHaveTextContent(`/tenant/${IDS.tenant}/event/${IDS.event}?lang=en`)
    },
}

export const WholeBallotBlank: Story = {
    args: {scenario: "blank"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const body = within(canvasElement.ownerDocument.body)
        await userEvent.click(await canvas.findByRole("button", {name: "Next"}))
        let dialog = within(
            await body.findByRole("dialog", {name: "You have not selected any candidates"})
        )
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(body.queryByRole("dialog")).not.toBeInTheDocument())
        await expect(store.getState().auditableBallots[IDS.election]).toBeUndefined()
        await userEvent.click(canvas.getByRole("button", {name: "Next"}))
        dialog = within(
            await body.findByRole("dialog", {name: "You have not selected any candidates"})
        )
        await userEvent.click(dialog.getByRole("button", {name: "Continue"}))
        await waitFor(() =>
            expect(body.getByRole("status", {name: "Current location"})).toHaveTextContent(
                `${electionPath}/review?lang=en`
            )
        )
        const decoded = decode_auditable_multi_ballot_js(
            store.getState().auditableBallots[IDS.election]!.auditableBallot
        ) as IDecodedVoteContest[]
        await expect(
            decoded.map(({contest_id, is_blank_ballot}) => ({contest_id, is_blank_ballot}))
        ).toEqual([
            {contest_id: IDS.contest, is_blank_ballot: true},
            {contest_id: "second-contest", is_blank_ballot: true},
        ])
        await expect(
            decoded.flatMap(({choices}) => choices.filter(({selected}) => selected >= 0))
        ).toEqual([])
    },
}

export const InvalidWasmInputShowsEncryptionError: Story = {
    args: {scenario: "wasm-error"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText("UNABLE_TO_ENCRYPT_BALLOT", {exact: true})
        ).toBeVisible()
        await expect(canvas.queryByRole("button", {name: "Next"})).not.toBeInTheDocument()
        await expect(store.getState().auditableBallots[IDS.election]).toBeUndefined()
    },
}

export const ExhaustedVotesLogOut: Story = {
    args: {scenario: "exhausted"},
    play: async () => {
        await waitFor(() => expect(logout).toHaveBeenCalled())
        await expect(store.getState().auditableBallots[IDS.election]).toBeUndefined()
    },
}
