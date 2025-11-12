// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    hashMultiBallot,
    hashBallot512,
    hashPlaintextBallot,
    decodeAuditableBallot,
    decodeAuditableMultiBallot,
    decodeAuditablePlaintextBallot,
    getLayoutProperties,
    getPoints,
    generateSampleAuditableBallot,
    checkIsBlank,
    IDecodedVoteContest,
    IDecodedVoteChoice,
    IBallotStyle,
    IContest,
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    IAuditablePlaintextBallot,
    IContestLayoutProperties,
    verifyBallotSignature,
    verifyMultiBallotSignature,
    verifyPlaintextBallotSignature,
} from "@sequentech/ui-core"

export interface IConfirmationBallot {
    ballot_hash: string
    election_config: IBallotStyle
    decoded_questions: Array<IDecodedVoteContest>
}

export interface IBallotService {
    hashMultiBallot: (auditableBallot: IAuditableMultiBallot) => string
    hashBallot512: (auditableBallot: IAuditableSingleBallot) => string
    hashPlaintextBallot: (auditableBallot: IAuditablePlaintextBallot) => string
    decodeAuditableBallot: (
        auditableBallot: IAuditableSingleBallot
    ) => Array<IDecodedVoteContest> | null
    decodeAuditableMultiBallot: (
        auditableBallot: IAuditableMultiBallot
    ) => Array<IDecodedVoteContest> | null
    decodeAuditablePlaintextBallot: (
        auditableBallot: IAuditablePlaintextBallot
    ) => Array<IDecodedVoteContest> | null
    getLayoutProperties: (question: IContest) => IContestLayoutProperties | null
    getPoints: (question: IContest, answer: IDecodedVoteChoice) => number | null
    generateSampleAuditableBallot: () => IAuditableSingleBallot | null
    checkIsBlank: (contest: IDecodedVoteContest) => boolean | null
    verifyBallotSignature: (
        ballot_id: string,
        election_id: string,
        content: IAuditableSingleBallot
    ) => boolean | null
    verifyMultiBallotSignature: (
        ballot_id: string,
        election_id: string,
        content: IAuditableMultiBallot
    ) => boolean | null
    verifyPlaintextBallotSignature: (
        ballot_id: string,
        election_id: string,
        content: IAuditablePlaintextBallot
    ) => boolean | null
}

export const provideBallotService = (): IBallotService => ({
    hashMultiBallot,
    hashBallot512,
    hashPlaintextBallot,
    decodeAuditableBallot,
    decodeAuditableMultiBallot,
    decodeAuditablePlaintextBallot,
    getLayoutProperties,
    getPoints,
    generateSampleAuditableBallot,
    checkIsBlank,
    verifyBallotSignature,
    verifyMultiBallotSignature,
    verifyPlaintextBallotSignature,
})
