// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {Component, ErrorInfo, ReactNode} from "react"
import {Meta, StoryObj} from "@storybook/react"
import {ContestDisplay, IContestDisplayProps, IBallotStyle} from "../ContestDisplay"
import {IContest, BallotSelection} from "@sequentech/ui-core"
import {IDecodedVoteContest} from "@sequentech/ui-core"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {Box} from "@mui/material"

import ballotStyle from "../data/ballotStyle.json"
import errorSelectionState from "../data/errorSelectionState.json"
import question from "../data/question.json"
import questionPlaintext from "../data/questionPlaintext.json"

const ContestDisplayWrapper: React.FC<IContestDisplayProps & {className?: string}> = ({
    className,
    ...props
}) => (
    <Box className={className}>
        <ContestDisplay {...props} />
    </Box>
)

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
