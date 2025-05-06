import { BallotSelection, IContest, IDecodedVoteChoice } from "@sequentech/ui-core"
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