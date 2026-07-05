// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EDeclineToVotePolicy, EElectionEventContestEncryptionPolicy} from "@sequentech/ui-core"
import {useAppSelector} from "../store/hooks"
import {selectElectionById} from "../store/elections/electionsSlice"
import {IBallotStyle, selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"

export function isMultiContestBallotStyle(ballotStyle?: IBallotStyle): boolean {
    return (
        ballotStyle?.ballot_eml.election_event_presentation?.contest_encryption_policy ===
        EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS
    )
}

export function useIsDeclineToVotePolicyEnabled(electionId?: string): boolean {
    const election = useAppSelector(selectElectionById(String(electionId)))
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))

    return (
        election?.presentation?.decline_to_vote_policy === EDeclineToVotePolicy.ENABLED &&
        isMultiContestBallotStyle(ballotStyle)
    )
}
