// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    toHashableBallot,
    toHashableMultiBallot,
    hashBallot,
    encryptBallotSelection,
    encryptMultiBallotSelection,
    interpretContestSelection,
    interpretMultiContestSelection,
    getWriteInAvailableCharacters,
    IHashableSingleBallot,
    IHashableMultiBallot,
    BallotSelection,
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
    toHashableBallot: (auditableBallot: IAuditableSingleBallot) => IHashableSingleBallot
    toHashableMultiBallot: (auditableBallot: IAuditableMultiBallot) => IHashableMultiBallot
    hashBallot: (auditableBallot: IAuditableSingleBallot) => string
    hashMultiBallot: (auditableBallot: IAuditableMultiBallot) => string
    encryptBallotSelection: (
        ballotSelection: BallotSelection,
        election: IBallotStyle
    ) => IAuditableSingleBallot
    encryptMultiBallotSelection: (
        ballotSelection: BallotSelection,
        election: IBallotStyle
    ) => IAuditableMultiBallot
    interpretContestSelection: (
        contestSelection: Array<IDecodedVoteContest>,
        election: IBallotStyle
    ) => Array<IDecodedVoteContest>
    interpretMultiContestSelection: (
        contestSelections: Array<IDecodedVoteContest>,
        election: IBallotStyle
    ) => Array<IDecodedVoteContest>
    getWriteInAvailableCharacters: (
        contestSelection: IDecodedVoteContest,
        election: IBallotStyle
    ) => number
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
    toHashableBallot,
    toHashableMultiBallot,
    hashBallot,
    hashMultiBallot,
    encryptBallotSelection,
    encryptMultiBallotSelection,
    interpretContestSelection,
    interpretMultiContestSelection,
    getWriteInAvailableCharacters,
    decodeAuditableBallot,
    decodeAuditableMultiBallot,
    checkIsBlank,
    hashBallot512,
    getLayoutProperties,
    getPoints,
    generateSampleAuditableBallot,
})
