// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    toHashableBallot,
    hashBallot,
    encryptBallotSelection,
    interpretContestSelection,
    getWriteInAvailableCharacters,
    decodeAuditableBallot,
    checkIsBlank,
    IDecodedVoteContest,
    IBallotStyle,
    IAuditableBallot,
    IHashableBallot,
    IContest,
    BallotSelection,
} from "@sequentech/ui-core"

export interface IBallotService {
    toHashableBallot: (auditableBallot: IAuditableBallot) => IHashableBallot
    hashBallot: (auditableBallot: IAuditableBallot) => string
    encryptBallotSelection: (
        ballotSelection: BallotSelection,
        election: IBallotStyle
    ) => IAuditableBallot
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
    hashBallot,
    encryptBallotSelection,
    interpretContestSelection,
    getWriteInAvailableCharacters,
    decodeAuditableBallot,
    checkIsBlank,
})
