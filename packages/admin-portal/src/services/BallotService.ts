// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    IAuditableBallot,
    IHashableBallot,
    IElectionDTO,
    hash_cyphertext_js,
    encrypt_decoded_question_js,
} from "sequent-core"
import {BallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"

export interface IBallotService {
    hashBallot256: (auditableBallot: IAuditableBallot) => string
    encryptBallotSelection: (
        ballotSelection: BallotSelection,
        election: IElectionDTO
    ) => IAuditableBallot
}

export const toHashableBallot = (auditableBallot: IAuditableBallot): IHashableBallot => ({
    choices: auditableBallot.choices.map((choice) => ({
        alpha: choice.alpha,
        beta: choice.beta,
    })),
    issue_date: auditableBallot.issue_date,
    proofs: auditableBallot.proofs,
})

export const hashBallot256 = (auditableBallot: IAuditableBallot): string => {
    try {
        return hash_cyphertext_js(toHashableBallot(auditableBallot))
    } catch (e) {
        console.log(e)
        throw e
    }
}

export const encryptBallotSelection = (
    ballotSelection: BallotSelection,
    election: IElectionDTO
): IAuditableBallot => {
    try {
        return encrypt_decoded_question_js(ballotSelection, election)
    } catch (e) {
        console.log(e)
        throw e
    }
}

export const provideBallotService = (): IBallotService => ({
    hashBallot256,
    encryptBallotSelection,
})
