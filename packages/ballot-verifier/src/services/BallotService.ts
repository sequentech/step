// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    hash_auditable_ballot_js,
    decode_auditable_ballot_js,
    get_layout_properties_from_contest_js,
    get_candidate_points_js,
    generate_sample_auditable_ballot_js,
    check_is_blank_js,
    IDecodedVoteContest,
    IContestLayoutProperties,
    IDecodedVoteChoice,
} from "sequent-core"
import {
    IBallotStyle,
    IContest,
    IAuditableBallot,
    IHashableBallot,
    CandidatesOrder,
} from "@sequentech/ui-essentials"
//import PlaintextVote from "../fixtures/plaintext_vote.json"

export interface IConfirmationBallot {
    ballot_hash: string
    election_config: IBallotStyle
    decoded_questions: Array<IDecodedVoteContest>
}

export interface IBallotService {
    hashBallot512: (auditableBallot: IAuditableBallot) => string
    decodeAuditableBallot: (auditableBallot: IAuditableBallot) => Array<IDecodedVoteContest> | null
    getLayoutProperties: (question: IContest) => IContestLayoutProperties | null
    getPoints: (question: IContest, answer: IDecodedVoteChoice) => number | null
    generateSampleAuditableBallot: () => IAuditableBallot | null
    checkIsBlank: (contest: IDecodedVoteContest) => boolean | null
}

export const hashBallot512 = (auditableBallot: IAuditableBallot): string => {
    try {
        return hash_auditable_ballot_js(auditableBallot)
    } catch (e) {
        console.log(e)
        throw e
    }
}

export const decodeAuditableBallot = (
    auditableBallot: IAuditableBallot
): Array<IDecodedVoteContest> | null => {
    try {
        let decodedBallot = decode_auditable_ballot_js(auditableBallot)
        return decodedBallot as Array<IDecodedVoteContest>
    } catch (error) {
        console.log(error)
        return null
    }
}

export const getLayoutProperties = (question: IContest): IContestLayoutProperties | null => {
    try {
        let properties = get_layout_properties_from_contest_js(question)
        return (properties || null) as IContestLayoutProperties | null
    } catch (error) {
        console.log(error)
        return null
    }
}

export const getPoints = (question: IContest, answer: IDecodedVoteChoice): number | null => {
    try {
        let points: number | undefined = get_candidate_points_js(question, answer)
        return points || null
    } catch (error) {
        console.log(error)
        return null
    }
}

export const generateSampleAuditableBallot = (): IAuditableBallot | null => {
    try {
        let auditableBallot: IAuditableBallot = generate_sample_auditable_ballot_js()
        return auditableBallot
    } catch (error) {
        console.log(error)
        return null
    }
}

export const checkIsBlank = (contest: IDecodedVoteContest): boolean | null => {
    try {
        let is_blank: boolean = check_is_blank_js(contest)
        return is_blank
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
    generateSampleAuditableBallot,
    checkIsBlank,
})
