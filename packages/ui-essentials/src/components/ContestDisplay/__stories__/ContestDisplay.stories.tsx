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

let votes: IDecodedVoteContest | undefined = undefined

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

    const state = (selectionState as unknown) as IDecodedVoteContest

    const ballotEmlContest = ballotStyle.ballot_eml.contests.find(
        (contest) => contest.id === contestId
    )
    // check bounds
    if (isUndefined(ballotEmlContest)) {
        votes = state
        console.log("votes 1", votes)
        return
    }

    console.log("state", state)

    let currentElection: IDecodedVoteContest[] = state[ballotStyle.election_id]
    let currentQuestion = currentElection?.find((contest) => contest.contest_id === contestId)
    let currentChoiceIndex = currentQuestion?.choices.findIndex(
        (choice) => voteChoice.id === choice.id
    )
    const currentChoice =
        !isUndefined(currentElection) && !isUndefined(currentChoiceIndex) && currentChoiceIndex > -1
            ? currentQuestion?.choices[currentChoiceIndex]
            : undefined

    console.log("ballotStyle.election_id", ballotStyle.election_id)
    console.log("currentElection", currentElection)
    console.log("currentQuestion", currentQuestion)
    console.log("currentChoiceIndex", currentChoiceIndex)
    console.log("currentChoice", currentChoice)

    // check election state
    if (!currentElection || isUndefined(currentChoice)) {
        votes = state
        console.log("votes 2", votes)
        return
    }

    // modify
    if (currentQuestion && !isUndefined(currentChoiceIndex)) {
        currentQuestion.choices[currentChoiceIndex] = voteChoice
    }

    votes = state
    console.log("votes 3", votes)
}

export const Vote: Story = {
    args: {
        ballotStyle: (ballotStyle as unknown) as IBallotStyle,
        question: (question as unknown) as IContest,
        isReview: false,
        errorSelectionState: (errorSelectionState as unknown) as BallotSelection,
        questionPlaintext: votes,
        isVotedState: false,
        onSetBallotSelectionVoteChoice: setBallotSelectionVoteChoice,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
