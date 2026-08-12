// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Where a ballot's marks come from, and where they go.
 *
 * The ballot used to read redux directly: `useAppSelector` for what a voter had
 * chosen, `useAppDispatch` for every click. That tied it to the voting portal's
 * store, which is why the Election Architect's preview had to draw *copies* of
 * these components instead of the components themselves — and a copy of a ballot
 * is a ballot that can differ from the one voters are handed.
 *
 * So the store became an interface with seven members, and each host implements
 * it: the portal over its redux slice, the wizard over local state. There is one
 * implementation of the ballot, and both are looking at it.
 *
 * **Why a context and not props.** The plan said "props", and for `Question` that
 * is what happened — its single read is now a prop. But `Answer` is a leaf,
 * rendered once per candidate, three levels down through `AnswersList`, and it
 * writes on every click. Threading six callbacks and two lookups through two
 * intermediate components would have meant twenty-odd new props whose only job is
 * to be forwarded, and every one of them a place to forward the wrong contest.
 * `ProfileContext` in the wizard is the same shape for the same reason: state a
 * leaf needs and nothing in between cares about.
 *
 * **The members mirror the portal's existing selectors and actions one for one**,
 * deliberately. `setChoice`/`setBlank`/`setInvalid`/`reset` take the same argument
 * objects as `setBallotSelectionVoteChoice`, `setBallotSelectionBlankVote`,
 * `setBallotSelectionInvalidVote` and `resetBallotSelection`. That makes the
 * portal's adapter a one-line forward per member, which is the cheapest possible
 * transcription and the least likely to be subtly wrong — the thing that matters
 * most when the code being moved has no coverage but the tests written for this
 * move.
 */

import {createContext, PropsWithChildren, useContext} from "react"
import type {IDecodedVoteChoice, IDecodedVoteContest} from "@sequentech/ui-core"

import type {IBallotStyle} from "./types"

/**
 * The encoder's own shapes, not a second description of them.
 *
 * The first version of this file declared structural `VoteChoice` and
 * `ContestSelection` interfaces, which compiled and was wrong in the way this whole
 * task exists to fix: a second definition of a platform type drifts from the first.
 * `tsc` caught it immediately — `ContestSelection` was not assignable to
 * `IDecodedVoteContest` where `checkIsBlank` expected one — so the aliases now point
 * at the real types and exist only to give them names a host can read.
 */
export type VoteChoice = IDecodedVoteChoice
export type ContestSelection = IDecodedVoteContest

export interface BallotSelectionPort {
    /** What the voter has marked on this contest, if anything. */
    contest(ballotStyle: IBallotStyle, contestId: string): ContestSelection | undefined

    /** One candidate's mark on this contest. */
    choice(
        ballotStyle: IBallotStyle,
        contestId: string,
        candidateId: string
    ): VoteChoice | undefined

    setChoice(input: {ballotStyle: IBallotStyle; contestId: string; voteChoice: VoteChoice}): void

    /**
     * Mark the blank option, which is not the same as marking nothing.
     *
     * A separate member rather than a `setChoice` with a flag, because the portal
     * reducer does more than set one choice: choosing blank clears the rest.
     */
    setBlank(input: {ballotStyle: IBallotStyle; contestId: string; candidateId: string}): void

    setInvalid(input: {
        ballotStyle: IBallotStyle
        contestId: string
        isExplicitInvalid: boolean
    }): void

    /** Start this contest — or the whole ballot — over. */
    reset(input: {ballotStyle: IBallotStyle; force?: boolean; contestId?: string}): void

    /**
     * Whether this election has already been voted in.
     *
     * Read by the warning list to decide whether a complaint is still actionable.
     * It was `useParams` plus a store read, which is why that component could not
     * render outside a router.
     */
    isVoted(electionId?: string): boolean

    /**
     * Prefix for a candidate's photograph.
     *
     * The portal's bucket URL, from its runtime settings. The wizard has no
     * bucket — its images are `data:` URIs held in the plan — so it supplies an
     * empty string, and this is why the value is injected rather than read from a
     * settings provider the wizard does not have.
     */
    imageBaseUrl: string
}

/**
 * A port that refuses rather than pretending.
 *
 * A ballot rendered with no provider is a bug in the host, and the useful
 * behaviour is to say so at the first click rather than silently drop the mark —
 * which would look, to a voter, exactly like a ballot that does not work.
 */
const missing = (): never => {
    throw new Error(
        "No BallotSelectionProvider above this ballot. A host has to supply one: " +
            "the voting portal adapts its redux store, the Election Architect uses local state."
    )
}

const NONE: BallotSelectionPort = {
    contest: () => undefined,
    choice: () => undefined,
    setChoice: missing,
    setBlank: missing,
    setInvalid: missing,
    reset: missing,
    isVoted: () => false,
    imageBaseUrl: "",
}

const BallotSelectionContext = createContext<BallotSelectionPort>(NONE)

export const BallotSelectionProvider = ({
    port,
    children,
}: PropsWithChildren<{port: BallotSelectionPort}>): React.JSX.Element => (
    <BallotSelectionContext.Provider value={port}>{children}</BallotSelectionContext.Provider>
)

/** The host's selection store, from inside the ballot. */
export const useBallotSelection = (): BallotSelectionPort => useContext(BallotSelectionContext)
