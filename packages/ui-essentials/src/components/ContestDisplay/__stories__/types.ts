// SPDX-FileCopyrightText: 2025 Enric Badia <enric@xtremis.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BallotSelection, IContest, IDecodedVoteChoice} from "@sequentech/ui-core"
import {ContestDisplay, IContestDisplayProps, IBallotStyle} from "../ContestDisplay"

export interface BallotSelectionsState {
    [electionId: string]: BallotSelection | undefined
}

export interface ActionProps {
    ballotStyle: IBallotStyle
    contestId: string
    voteChoice: IDecodedVoteChoice
}

export interface ActionResetProps {
    ballotStyle: IBallotStyle
    force: boolean
}

export interface VoteStoryProps {
    ballotStyle: IBallotStyle
    question: IContest
    isReview: boolean
    errorSelectionState: BallotSelection
}
