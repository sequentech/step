// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {Component, ErrorInfo, ReactNode, useState} from "react"
import {Meta, StoryObj} from "@storybook/react"
import {ContestDisplay, IContestDisplayProps, IBallotStyle} from "../ContestDisplay"
import {IContest, BallotSelection, IDecodedVoteChoice, isUndefined} from "@sequentech/ui-core"
import {IDecodedVoteContest} from "@sequentech/ui-core"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {Box} from "@mui/material"

import ballotStyle from "../data/ballotStyle.json"
import errorSelectionState from "../data/errorSelectionState.json"
import question from "../data/question.json"
import questionPlaintext from "../data/questionPlaintext.json"
import selectionState from "../data/selectionState.json"

interface BallotSelectionsState {
    [electionId: string]: BallotSelection | undefined
}

let votes: BallotSelectionsState | undefined = undefined
const electionId = "1ae13934-8de6-47bc-8061-57280978e621"
const contestId = "6ee6b91f-63a0-4f13-a8ed-8f2defe4bc8f"

const ContestDisplayWrapper: React.FC<IContestDisplayProps & {className?: string}> = ({
    className,
    ...props
}) => {
    return (
        <Box className={className}>
            <ContestDisplay {...props} />
        </Box>
    )
}

const meta: Meta<typeof ContestDisplayWrapper> = {
    title: "components/ContestDisplay",
    component: ContestDisplayWrapper,
    parameters: {
        backgrounds: {
            default: "white",
        },
        viewport: {
            viewports: INITIAL_VIEWPORTS,
            defaultViewport: "iphone6",
        },
    },
}

export default meta

type Story = StoryObj<typeof ContestDisplayWrapper>

export const Primary: Story = {
    args: {
        ballotStyle: (ballotStyle as unknown) as IBallotStyle,
        question: (question as unknown) as IContest,
        isReview: true,
        errorSelectionState: (errorSelectionState as unknown) as BallotSelection,
        questionPlaintext: (questionPlaintext as unknown) as IDecodedVoteContest,
        isVotedState: false, // Adding the required isVotedState prop
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

interface ActionProps {
    ballotStyle: IBallotStyle
    contestId: string
    voteChoice: IDecodedVoteChoice
}

const setBallotSelectionVoteChoice = (action: ActionProps) => {
    const {ballotStyle, contestId, voteChoice} = action

    const state = (selectionState as unknown) as BallotSelectionsState | undefined

    const ballotEmlContest = ballotStyle.ballot_eml.contests.find(
        (contest) => contest.id === contestId
    )
    // check bounds
    if (isUndefined(ballotEmlContest)) {
        votes = state
        return
    }

    let currentElection: IDecodedVoteContest[] | undefined =
        state?.[ballotStyle.election_id] ?? undefined
    let currentQuestion = currentElection?.find((contest) => contest.contest_id === contestId)
    let currentChoiceIndex = currentQuestion?.choices.findIndex(
        (choice) => voteChoice.id === choice.id
    )
    const currentChoice =
        !isUndefined(currentElection) && !isUndefined(currentChoiceIndex) && currentChoiceIndex > -1
            ? currentQuestion?.choices[currentChoiceIndex]
            : undefined

    // check election state
    if (!currentElection || isUndefined(currentChoice)) {
        votes = state
        return
    }

    // modify
    if (currentQuestion && !isUndefined(currentChoiceIndex)) {
        currentQuestion.choices[currentChoiceIndex] = voteChoice
    }

    votes = state
}

// Create a proper React component to use hooks
const VoteStory: React.FC = ({...args}) => {
    // Use useState to track votes and force re-renders
    const [votesState, setVotesState] = useState<BallotSelectionsState | undefined>(undefined)

    // Custom handler that updates state after setting votes
    const handleSetBallotSelectionVoteChoice = (action: ActionProps) => {
        setBallotSelectionVoteChoice(action)
        // Update state with the new votes value to trigger re-render
        setVotesState({...votes})
    }

    // Get the current questionPlaintext value
    const currentQuestionPlaintext = votesState
        ? (votesState[electionId] as IDecodedVoteContest[])?.find((a) => a.contest_id === contestId)
        : undefined

    return (
        <ContestDisplayWrapper
            ballotStyle={(ballotStyle as unknown) as IBallotStyle}
            question={(question as unknown) as IContest}
            isReview={false}
            errorSelectionState={(errorSelectionState as unknown) as BallotSelection}
            questionPlaintext={currentQuestionPlaintext}
            isVotedState={false}
            onSetBallotSelectionVoteChoice={handleSetBallotSelectionVoteChoice}
        />
    )
}

export const Vote: Story = {
    render: () => <VoteStory />,
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
