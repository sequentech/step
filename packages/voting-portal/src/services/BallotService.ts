// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    IBallotStyle,
    to_hashable_ballot_js,
    hash_auditable_ballot_js,
    encrypt_decoded_question_js,
} from "sequent-core"
import {BallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"

export interface IBallotService {
    toHashableBallot: (auditableBallot: string) => string
    hashBallot: (auditableBallot: string) => string
    encryptBallotSelection: (ballotSelection: BallotSelection, election: IBallotStyle) => string
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
        return encrypt_decoded_question_js(ballotSelection, election)
    } catch (e) {
        console.log(e)
        throw e
    }
}

export const provideBallotService = (): IBallotService => ({
    toHashableBallot,
    hashBallot,
    encryptBallotSelection,
})
