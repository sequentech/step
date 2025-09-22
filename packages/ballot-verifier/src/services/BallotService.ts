// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    hashMultiBallot,
    hashBallot512,
    decodeAuditableBallot,
    decodeAuditableMultiBallot,
    getLayoutProperties,
    getPoints,
    generateSampleAuditableBallot,
    checkIsBlank,
    IDecodedVoteContest,
    IDecodedVoteChoice,
    IBallotStyle,
    IContest,
    IAuditableBallot,
    IAuditableSingleBallot,
    IContestLayoutProperties,
    IAuditableMultiBallot,
} from "@sequentech/ui-core"

export interface IConfirmationBallot {
    ballot_hash: string
    election_config: IBallotStyle
    decoded_questions: Array<IDecodedVoteContest>
}

export interface IBallotService {
    hashMultiBallot: (auditableBallot: IAuditableMultiBallot) => string
    hashBallot512: (auditableBallot: IAuditableSingleBallot) => string
    decodeAuditableBallot: (
        auditableBallot: IAuditableSingleBallot
    ) => Array<IDecodedVoteContest> | null
    decodeAuditableMultiBallot: (
        auditableBallot: IAuditableMultiBallot
    ) => Array<IDecodedVoteContest> | null
    getLayoutProperties: (question: IContest) => IContestLayoutProperties | null
    getPoints: (question: IContest, answer: IDecodedVoteChoice) => number | null
    generateSampleAuditableBallot: () => IAuditableSingleBallot | null
    checkIsBlank: (contest: IDecodedVoteContest) => boolean | null
}

export const provideBallotService = (): IBallotService => ({
    hashMultiBallot,
    hashBallot512,
    decodeAuditableBallot,
    decodeAuditableMultiBallot,
    getLayoutProperties,
    getPoints,
    generateSampleAuditableBallot,
    checkIsBlank,
})
