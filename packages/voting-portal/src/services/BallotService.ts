// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    IBallotStyle,
    to_hashable_ballot_js,
    hash_auditable_ballot_js,
    encrypt_decoded_contest_js,
    test_contest_reencoding_js,
    IDecodedVoteContest,
} from "sequent-core"
import {BallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"

export interface IBallotService {
    toHashableBallot: (auditableBallot: string) => string
    hashBallot: (auditableBallot: string) => string
    encryptBallotSelection: (ballotSelection: BallotSelection, election: IBallotStyle) => string
    interpretContestSelection: (
        contestSelection: IDecodedVoteContest,
        election: IBallotStyle
    ) => IDecodedVoteContest
}

export const toHashableBallot = (auditableBallot: string): string => {
    try {
        return to_hashable_ballot_js(auditableBallot)
    } catch (e) {
        console.log(e)
        throw e
    }
}

export const hashBallot = (auditableBallot: string): string => {
    try {
        return hash_auditable_ballot_js(auditableBallot)
    } catch (e) {
        console.log(e)
        throw e
    }
}

export const encryptBallotSelection = (
    ballotSelection: BallotSelection,
    election: IBallotStyle
): string => {
    try {
        return encrypt_decoded_contest_js(ballotSelection, election)
    } catch (e) {
        console.log(e)
        throw e
    }
}

/*
 * Encodes and decodes the contest selection.
 * The result is getting the ballot selection back from sequent-core,
 * but this time with the invalid errors. Also this allows the system
 * to check that the ballot selection is the same.
 */
export const interpretContestSelection = (
    contestSelection: IDecodedVoteContest,
    election: IBallotStyle
): IDecodedVoteContest => {
    try {
        return test_contest_reencoding_js(contestSelection, election)
    } catch (e) {
        console.log(e)
        throw e
    }
}

export const provideBallotService = (): IBallotService => ({
    toHashableBallot,
    hashBallot,
    encryptBallotSelection,
    interpretContestSelection,
})
