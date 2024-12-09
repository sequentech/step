// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    toHashableBallot,
    toHashableMultiBallot,
    hashBallot,
    hashMultiBallot,
    encryptBallotSelection,
    encryptMultiBallotSelection,
    interpretContestSelection,
    getWriteInAvailableCharacters,
    decodeAuditableBallot,
    decodeAuditableMultiBallot,
    checkIsBlank,
    IDecodedVoteContest,
    IBallotStyle,
    IAuditableBallot,
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    IHashableBallot,
    IHashableSingleBallot,
    IHashableMultiBallot,
    IContest,
    BallotSelection,
} from "@sequentech/ui-core"

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
        contestSelection: IDecodedVoteContest,
        election: IBallotStyle
    ) => IDecodedVoteContest
    getWriteInAvailableCharacters: (
        contestSelection: IDecodedVoteContest,
        election: IBallotStyle
    ) => number
    decodeAuditableBallot: (
        auditableBallot: IAuditableSingleBallot
    ) => Array<IDecodedVoteContest> | null
    decodeAuditableMultiBallot: (
        auditableBallot: IAuditableMultiBallot
    ) => Array<IDecodedVoteContest> | null
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
    getWriteInAvailableCharacters,
    decodeAuditableBallot,
    decodeAuditableMultiBallot,
    checkIsBlank,
})
