// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo} from "react"
import {Provider} from "react-redux"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within, waitFor} from "storybook/test"
import {
    initCore,
    CandidatesOrder,
    ICountingAlgorithm,
    type IBallotStyle as BallotDefinition,
    EInvalidVotePolicy,
    EOverVotePolicy,
    ECandidatesSelectionPolicy,
    ECollapsibleLists,
} from "@sequentech/ui-core"
import {electionFixture} from "@sequentech/ui-test-kit/fixtures"
import {Question} from "../Question"
import {clearVoterSession, store} from "../../../store/store"
import {useAppSelector} from "../../../store/hooks"
import {
    resetBallotSelection,
    selectBallotSelectionByElectionId,
} from "../../../store/ballotSelections/ballotSelectionsSlice"
import type {IBallotStyle} from "../../../store/ballotStyles/ballotStylesSlice"
import {provideBallotService} from "../../../services/BallotService"

type Scenario =
    | "overvote"
    | "disable"
    | "minimum"
    | "invalid-exclusive"
    | "invalid-inclusive"
    | "blank"
    | "radio"
    | "acclaimed"
    | "categories"
    | "review-blank"
    | "review-decline"
    | "preferential"
    | "write-in"

function fixture(scenario: Scenario): IBallotStyle {
    const ballot = electionFixture().ballot as unknown as BallotDefinition
    const contest = ballot.contests[0]
    contest.min_votes = 0
    contest.presentation = {candidates_order: CandidatesOrder.CUSTOM}
    contest.candidates.push({
        ...contest.candidates[1],
        id: "candidate-charlie",
        name: "Charlie Example",
        presentation: {sort_order: 2},
    })
    if (scenario === "disable") {
        contest.max_votes = 2
        contest.presentation.over_vote_policy = EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE
    }
    if (scenario === "minimum") {
        contest.min_votes = 2
        contest.max_votes = 2
        contest.presentation.invalid_vote_policy = EInvalidVotePolicy.NOT_ALLOWED
    }
    if (scenario === "radio")
        contest.presentation.candidates_selection_policy = ECandidatesSelectionPolicy.RADIO
    if (scenario.startsWith("invalid")) {
        contest.presentation.invalid_vote_policy =
            scenario === "invalid-exclusive"
                ? EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT
                : EInvalidVotePolicy.ALLOWED
        contest.candidates.push({
            ...contest.candidates[0],
            id: "candidate-invalid",
            name: "Invalid vote",
            presentation: {is_explicit_invalid: true, sort_order: 3},
        })
    }
    if (scenario === "blank")
        contest.candidates.push({
            ...contest.candidates[0],
            id: "candidate-blank",
            name: "Blank vote",
            presentation: {is_explicit_blank: true, sort_order: 3},
        })
    if (scenario === "acclaimed") contest.is_acclaimed = true
    if (scenario === "preferential") {
        contest.counting_algorithm = ICountingAlgorithm.INSTANT_RUNOFF
        contest.max_votes = 3
    }
    if (scenario === "write-in") {
        contest.presentation.allow_writeins = true
        contest.candidates.push({
            ...contest.candidates[0],
            id: "candidate-write-in",
            name: "Another candidate",
            presentation: {is_write_in: true, sort_order: 3},
        })
    }
    if (scenario === "categories") {
        contest.presentation.collapsible_lists = ECollapsibleLists.ENABLED_COLLAPSED
        contest.candidates[0].candidate_type = "__proto__"
        contest.candidates[1].candidate_type = "constructor"
        contest.candidates[2].candidate_type = "Council list"
    }
    return {
        id: ballot.id,
        election_id: ballot.election_id,
        tenant_id: ballot.tenant_id,
        election_event_id: ballot.election_event_id,
        ballot_eml: ballot,
        created_at: "2026-01-01T00:00:00Z",
        last_updated_at: "2026-01-01T00:00:00Z",
    }
}

function QuestionView({ballotStyle, scenario}: {ballotStyle: IBallotStyle; scenario: Scenario}) {
    const selection = useAppSelector(selectBallotSelectionByElectionId(ballotStyle.election_id))
    const errors = useMemo(
        () =>
            provideBallotService().interpretContestSelection(
                selection ?? [],
                ballotStyle.ballot_eml
            ),
        [selection, ballotStyle]
    )
    return (
        <Question
            ballotStyle={ballotStyle}
            question={ballotStyle.ballot_eml.contests[0]}
            isReview={scenario.startsWith("review")}
            isBlankBallot={scenario === "review-blank"}
            isDeclineToVote={scenario === "review-decline"}
            errorSelectionState={errors}
            setDecodedContests={() => {}}
        />
    )
}

const meta = {
    title: "Ballot/Question",
    args: {scenario: "overvote" as Scenario},
    loaders: [
        async ({args}) => {
            await initCore()
            store.dispatch(clearVoterSession())
            const ballotStyle = fixture(args.scenario)
            store.dispatch(resetBallotSelection({ballotStyle, force: true}))
            return {ballotStyle}
        },
    ],
    render: (args, {loaded}) => (
        <Provider store={store}>
            <main>
                <h1>Voting ballot</h1>
                <QuestionView
                    scenario={args.scenario}
                    ballotStyle={loaded.ballotStyle as IBallotStyle}
                />
            </main>
        </Provider>
    ),
} satisfies Meta<{scenario: Scenario}>
export default meta
type Story = StoryObj<typeof meta>

export const DefaultOvervote: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("checkbox", {name: /Alice Example/}))
        await userEvent.click(canvas.getByRole("checkbox", {name: /Bob Example/}))
        await expect(canvas.getByRole("checkbox", {name: /Alice Example/})).toBeChecked()
        await expect(canvas.getByRole("checkbox", {name: /Bob Example/})).toBeChecked()
        await expect(
            canvas.getByText("Overvote: Number of selected choices 2 is more than the maximum 1")
        ).toBeVisible()
    },
}

export const DisableAtMaximum: Story = {
    args: {scenario: "disable"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const alice = canvas.getByRole("checkbox", {name: /Alice Example/})
        const charlie = canvas.getByRole("checkbox", {name: /Charlie Example/})
        await userEvent.click(alice)
        await userEvent.click(canvas.getByRole("checkbox", {name: /Bob Example/}))
        await expect(charlie).toBeDisabled()
        await userEvent.click(alice)
        await expect(charlie).toBeEnabled()
        await userEvent.click(charlie)
        await expect(charlie).toBeChecked()
    },
}

export const BelowMinimum: Story = {
    args: {scenario: "minimum"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("checkbox", {name: /Alice Example/}))
        await expect(
            canvas.getByText("Number of selected choices 1 is less than the minimum 2")
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("checkbox", {name: /Bob Example/}))
        await expect(
            canvas.queryByText("Number of selected choices 1 is less than the minimum 2")
        ).not.toBeInTheDocument()
    },
}

export const ExclusiveInvalid: Story = {
    args: {scenario: "invalid-exclusive"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const alice = canvas.getByRole("checkbox", {name: /Alice Example/})
        const invalid = canvas.getByRole("checkbox", {name: /Invalid vote/})
        await userEvent.click(alice)
        await userEvent.click(invalid)
        await expect(invalid).toBeChecked()
        await expect(alice).not.toBeChecked()
        await userEvent.click(alice)
        await expect(alice).toBeChecked()
        await expect(invalid).not.toBeChecked()
    },
}

export const InclusiveInvalid: Story = {
    args: {scenario: "invalid-inclusive"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const alice = canvas.getByRole("checkbox", {name: /Alice Example/})
        const invalid = canvas.getByRole("checkbox", {name: /Invalid vote/})
        await userEvent.click(alice)
        await userEvent.click(invalid)
        await expect(alice).toBeChecked()
        await expect(invalid).toBeChecked()
    },
}

export const ExclusiveBlank: Story = {
    args: {scenario: "blank"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const alice = canvas.getByRole("checkbox", {name: /Alice Example/})
        const blank = canvas.getByRole("checkbox", {name: /Blank vote/})
        await userEvent.click(alice)
        await userEvent.click(blank)
        await expect(blank).toBeChecked()
        await expect(alice).not.toBeChecked()
        await userEvent.click(alice)
        await expect(alice).toBeChecked()
        await expect(blank).not.toBeChecked()
    },
}

export const RadioSelection: Story = {
    args: {scenario: "radio"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("checkbox", {name: /Alice Example/}))
        await userEvent.click(canvas.getByRole("checkbox", {name: /Bob Example/}))
        await expect(canvas.getByRole("checkbox", {name: /Alice Example/})).not.toBeChecked()
        await expect(canvas.getByRole("checkbox", {name: /Bob Example/})).toBeChecked()
    },
}

export const AcclaimedContest: Story = {
    args: {scenario: "acclaimed"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/This contest was decided by acclamation/)).toBeVisible()
        for (const checkbox of canvas.getAllByRole("checkbox"))
            await expect(checkbox).toBeDisabled()
        await expect(canvas.queryAllByRole("combobox")).toHaveLength(0)
    },
}

export const CollapsibleCategoryNames: Story = {
    args: {scenario: "categories"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("button", {name: "Expand all"})).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Expand all"}))
        await waitFor(() => expect(canvas.getByText("Alice Example", {exact: true})).toBeVisible())
        await expect(canvas.getByRole("checkbox", {name: /Bob Example/})).toBeEnabled()
        await userEvent.click(canvas.getByRole("button", {name: "Collapse all"}))
        await waitFor(() =>
            expect(canvas.getByText("Alice Example", {exact: true})).not.toBeVisible()
        )
    },
}

export const BlankBallotReview: Story = {
    args: {scenario: "review-blank"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/blank ballot/i)).toBeVisible()
        await expect(canvas.queryAllByRole("checkbox")).toHaveLength(0)
    },
}

export const DeclinedBallotReview: Story = {
    args: {scenario: "review-decline"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/decline to vote/i)).toBeVisible()
        await expect(canvas.queryAllByRole("checkbox")).toHaveLength(0)
    },
}

export const PreferentialDuplicateThenGap: Story = {
    args: {scenario: "preferential"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const body = within(canvasElement.ownerDocument.body)
        const alice = canvas.getByRole("combobox", {name: /Alice Example/})
        const bob = canvas.getByRole("combobox", {name: /Bob Example/})
        await userEvent.click(alice)
        await userEvent.click(body.getByRole("option", {name: "1st"}))
        await userEvent.click(bob)
        await userEvent.click(body.getByRole("option", {name: "1st"}))
        await expect(
            canvas.getByText(/same position was selected for two or more candidates/i)
        ).toBeVisible()
        await userEvent.click(bob)
        await userEvent.click(body.getByRole("option", {name: "3rd"}))
        await expect(alice).toHaveTextContent("1st")
        await expect(bob).toHaveTextContent("3rd")
        await expect(canvas.getByText(/order of preference has one or more gaps/i)).toBeVisible()
        await userEvent.click(bob)
        await userEvent.click(body.getByRole("option", {name: "2nd"}))
        await expect(
            canvas.queryByText(/order of preference has one or more gaps/i)
        ).not.toBeInTheDocument()
        await expect(alice).toHaveTextContent("1st")
    },
}

export const WriteInBudgetAnnounced: Story = {
    args: {scenario: "write-in"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const input = canvas.getByRole("textbox", {name: /Another candidate/})
        await userEvent.type(input, "Z".repeat(100))
        const error = await canvas.findByText(/write-in exceeds the maximum length/i)
        await expect(error).toBeVisible()
        await expect(input).toHaveAccessibleDescription(/write-in exceeds the maximum length/i)
        await userEvent.clear(input)
        await userEvent.type(input, "DANA EXAMPLE")
        await expect(
            canvas.queryByText(/write-in exceeds the maximum length/i)
        ).not.toBeInTheDocument()
        await expect(input).toHaveValue("DANA EXAMPLE")
    },
}
