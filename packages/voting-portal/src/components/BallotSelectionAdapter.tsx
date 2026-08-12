// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * This portal's redux store, as the shared ballot's selection port.
 *
 * The ballot components — `Question`, `Answer`, `AnswersList`,
 * `InvalidErrorsList` — moved into `@sequentech/ui-essentials` so the Election
 * Architect's preview can render the *same* components a voter is handed, rather
 * than copies of them. They no longer know about redux; they ask a port for the
 * voter's marks and hand changes back to it. This is the portal's implementation
 * of that port, and it is deliberately nothing but forwarding.
 *
 * **Every member is a one-line forward to the selector or action it replaced**, in
 * the same argument shape. That was the point of designing the port to mirror the
 * slice: the transcription is checkable by eye, which matters because the code
 * being moved has no coverage other than the tests written for the move.
 *
 * | port member    | was                                       |
 * | -------------- | ----------------------------------------- |
 * | `contest`      | `selectBallotSelectionQuestion`           |
 * | `choice`       | `selectBallotSelectionVoteChoice`         |
 * | `setChoice`    | `setBallotSelectionVoteChoice`            |
 * | `setBlank`     | `setBallotSelectionBlankVote`             |
 * | `setInvalid`   | `setBallotSelectionInvalidVote`           |
 * | `reset`        | `resetBallotSelection`                    |
 * | `isVoted`      | `isVotedByElectionId` + `useParams`       |
 * | `imageBaseUrl` | `SettingsContext`'s `PUBLIC_BUCKET_URL`   |
 *
 * `isVoted` is the one that changed shape rather than moving: the warning list used
 * to read the election id off the URL with `useParams`, which meant a list of
 * warnings could not render outside a `RouterProvider`. It takes the id now.
 */

import {PropsWithChildren, useContext, useMemo} from "react"
import {BallotSelectionProvider} from "@sequentech/ui-essentials"
import type {BallotSelectionPort} from "@sequentech/ui-essentials"
import {useStore} from "react-redux"

import {useAppDispatch, useAppSelector} from "../store/hooks"
import {
    resetBallotSelection,
    selectBallotSelectionQuestion,
    selectBallotSelectionVoteChoice,
    setBallotSelectionBlankVote,
    setBallotSelectionInvalidVote,
    setBallotSelectionVoteChoice,
} from "../store/ballotSelections/ballotSelectionsSlice"
import {isVotedByElectionId} from "../store/extra/extraSlice"
import {SettingsContext} from "../providers/SettingsContextProvider"
import type {RootState} from "../store/store"

export const BallotSelectionAdapter = ({children}: PropsWithChildren): React.JSX.Element => {
    const dispatch = useAppDispatch()
    const {globalSettings} = useContext(SettingsContext)

    // Subscribed once, at the provider, so a change re-renders the ballot below it.
    // `contest` and `choice` then read through `store.getState()` rather than
    // through their own `useAppSelector` — a hook cannot be called from inside a
    // port method, because the number of candidates decides how many there would
    // be, and that is exactly the rule hooks have.
    const selections = useAppSelector((state: RootState) => state.ballotSelections)
    const voted = useAppSelector((state: RootState) => state.extra)
    const store = useStore<RootState>()

    const port = useMemo<BallotSelectionPort>(
        () => ({
            contest: (ballotStyle, contestId) =>
                selectBallotSelectionQuestion(ballotStyle.election_id, contestId)(store.getState()),
            choice: (ballotStyle, contestId, candidateId) =>
                selectBallotSelectionVoteChoice(
                    ballotStyle.election_id,
                    contestId,
                    candidateId
                )(store.getState()),
            setChoice: (input) => {
                dispatch(setBallotSelectionVoteChoice(input))
            },
            setBlank: (input) => {
                dispatch(setBallotSelectionBlankVote(input))
            },
            setInvalid: (input) => {
                dispatch(setBallotSelectionInvalidVote(input))
            },
            reset: (input) => {
                dispatch(resetBallotSelection(input))
            },
            isVoted: (electionId) => isVotedByElectionId(electionId)(store.getState()),
            imageBaseUrl: globalSettings.PUBLIC_BUCKET_URL ?? "",
        }),
        // `selections` and `voted` are in the list although the body does not name
        // them: they are what makes a new port object — and so a re-render of the
        // ballot — happen when a mark changes. Reading them through `getState()`
        // alone would subscribe to nothing and the ballot would not update.
        [dispatch, store, globalSettings.PUBLIC_BUCKET_URL, selections, voted]
    )

    return <BallotSelectionProvider port={port}>{children}</BallotSelectionProvider>
}
