// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    hash_ballot,
    check_ballot_format,
    map_to_decoded_ballot,
    get_layout_properties_from_question,
    get_answer_points,
} from "new-ballot-verifier-lib"
import {
    IAuditableBallot,
    IElectionDTO,
    IDecodedVoteQuestion,
    IQuestion,
    IQuestionLayoutProperties,
    IDecodedVoteChoice,
} from "sequent-core"
//import PlaintextVote from "../fixtures/plaintext_vote.json"

export interface IConfirmationBallot {
    ballot_hash: string
    election_config: IElectionDTO
    decoded_questions: Array<IDecodedVoteQuestion>
}

export interface IBallotService {
    hashBallot512: (auditableBallot: IAuditableBallot) => string
    parseAuditableBallotJSON: (json: any) => IAuditableBallot | null
    decodeAuditableBallot: (auditableBallot: IAuditableBallot) => Array<IDecodedVoteQuestion> | null
    getLayoutProperties: (question: IQuestion) => IQuestionLayoutProperties | null
    getPoints: (question: IQuestion, answer: IDecodedVoteChoice) => number | null
}

export const hashBallot512 = (auditableBallot: IAuditableBallot): string => {
    try {
        return hash_ballot(auditableBallot)
    } catch (e) {
        console.log(e)
        throw e
    }
}

export const parseAuditableBallotJSON = (json: any): IAuditableBallot | null => {
    try {
        let result = check_ballot_format(json)
        if (!result) {
            return null
        }
        return json as IAuditableBallot
    } catch (error) {
        console.log(error)
        return null
    }
}

export const decodeAuditableBallot = (
    auditableBallot: IAuditableBallot
): Array<IDecodedVoteQuestion> | null => {
    try {
        let decodedBallot = map_to_decoded_ballot(auditableBallot)
        return decodedBallot as Array<IDecodedVoteQuestion>
    } catch (error) {
        console.log(error)
        return null
    }
}

export const getLayoutProperties = (question: IQuestion): IQuestionLayoutProperties | null => {
    try {
        let properties = get_layout_properties_from_question(question)
        return (properties || null) as IQuestionLayoutProperties | null
    } catch (error) {
        console.log(error)
        return null
    }
}

export const getPoints = (question: IQuestion, answer: IDecodedVoteChoice): number | null => {
    try {
        let points: number | undefined = get_answer_points(question, answer)
        return points || null
    } catch (error) {
        console.log(error)
        return null
    }
}

export const provideBallotService = (): IBallotService => ({
    hashBallot512,
    parseAuditableBallotJSON,
    decodeAuditableBallot,
    getLayoutProperties,
    getPoints,
})
