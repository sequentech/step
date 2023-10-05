// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    hash_auditable_ballot_js,
    decode_auditable_ballot_js,
    get_layout_properties_from_question_js,
    get_answer_points_js,
    get_ballot_style_from_auditable_ballot_js,
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
    hashBallot512: (auditableBallot: string) => string
    decodeAuditableBallot: (auditableBallot: string) => Array<IDecodedVoteQuestion> | null
    getLayoutProperties: (question: IQuestion) => IQuestionLayoutProperties | null
    getPoints: (question: IQuestion, answer: IDecodedVoteChoice) => number | null
    getBallotStyleFromAuditableBallot: (
        auditableBallot: string
    ) => IElectionDTO | null
}

export const hashBallot512 = (auditableBallot: string): string => {
    try {
        return hash_auditable_ballot_js(auditableBallot)
    } catch (e) {
        console.log(e)
        throw e
    }
}

export const decodeAuditableBallot = (
    auditableBallot: string
): Array<IDecodedVoteQuestion> | null => {
    try {
        let decodedBallot = decode_auditable_ballot_js(auditableBallot)
        return decodedBallot as Array<IDecodedVoteQuestion>
    } catch (error) {
        console.log(error)
        return null
    }
}

export const getBallotStyleFromAuditableBallot = (
    auditableBallot: string
): IElectionDTO | null => {
    try {
        let ballotStyle = get_ballot_style_from_auditable_ballot_js(auditableBallot) as IElectionDTO
        return ballotStyle
    } catch (error) {
        console.log(error)
        return null
    }
}

export const getLayoutProperties = (question: IQuestion): IQuestionLayoutProperties | null => {
    try {
        let properties = get_layout_properties_from_question_js(question)
        return (properties || null) as IQuestionLayoutProperties | null
    } catch (error) {
        console.log(error)
        return null
    }
}

export const getPoints = (question: IQuestion, answer: IDecodedVoteChoice): number | null => {
    try {
        let points: number | undefined = get_answer_points_js(question, answer)
        return points || null
    } catch (error) {
        console.log(error)
        return null
    }
}

export const provideBallotService = (): IBallotService => ({
    hashBallot512,
    decodeAuditableBallot,
    getLayoutProperties,
    getPoints,
    getBallotStyleFromAuditableBallot,
})
