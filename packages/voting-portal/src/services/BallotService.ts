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
    checkIsBlank,
    IDecodedVoteContest,
    IBallotStyle,
    IAuditableBallot,
    IAuditableMultiBallot,
    IHashableBallot,
    IHashableMultiBallot,
    IContest,
    BallotSelection,
} from "@sequentech/ui-core"

export interface IBallotService {
    toHashableBallot: (auditableBallot: IAuditableBallot) => IHashableBallot
    toHashableMultiBallot: (auditableBallot: IAuditableMultiBallot) => IHashableMultiBallot
    hashBallot: (auditableBallot: IAuditableBallot) => string
    hashMultiBallot: (auditableBallot: IAuditableMultiBallot) => string
    encryptBallotSelection: (
        ballotSelection: BallotSelection,
        election: IBallotStyle
    ) => IAuditableBallot
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
    decodeAuditableBallot: (auditableBallot: IAuditableBallot) => Array<IDecodedVoteContest> | null
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
    checkIsBlank,
})
