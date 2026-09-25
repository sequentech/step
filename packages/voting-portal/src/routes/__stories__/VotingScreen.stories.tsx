// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Provider} from "react-redux"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within, waitFor} from "storybook/test"
import {
    initCore,
    type IBallotStyle as BallotDefinition,
    CandidatesOrder,
    EInvalidVotePolicy,
    EBlankVotePolicy,
    EConsolidatedReportPolicy,
} from "@sequentech/ui-core"
import {electionFixture, IDS, FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import VotingScreen, {action} from "../VotingScreen"
import {clearVoterSession, store} from "../../store/store"
import {setElection, type IElectionExtended} from "../../store/elections/electionsSlice"
import {setBallotStyle, type IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {resetBallotSelection} from "../../store/ballotSelections/ballotSelectionsSlice"

type Scenario = "blocked" | "pagination" | "warning" | "back-start" | "back-chooser"
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
    if (scenario === "pagination") {
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
        num_allowed_revotes: 0,
        presentation: {
            consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
            voting_screen_back_policy:
                scenario === "back-start" ? "start-screen" : "election-selection-screen",
        },
    }
    store.dispatch(clearVoterSession())
    store.dispatch(setElection(election))
    store.dispatch(setBallotStyle(ballotStyle))
    store.dispatch(resetBallotSelection({ballotStyle, force: true}))
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
                <VotingScreen />
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
