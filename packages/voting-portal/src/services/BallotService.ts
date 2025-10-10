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
    interpretMultiContestSelection,
    getWriteInAvailableCharacters,
    decodeAuditableBallot,
    decodeAuditableMultiBallot,
    checkIsBlank,
    signHashableBallot,
    signHashableMultiBallot,
    IDecodedVoteContest,
    IBallotStyle,
    IAuditableBallot,
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    IHashableBallot,
    IHashableSingleBallot,
    IHashableMultiBallot,
    ISignedContent,
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
    decodeAuditableBallot: (
        auditableBallot: IAuditableSingleBallot
    ) => Array<IDecodedVoteContest> | null
    decodeAuditableMultiBallot: (
        auditableBallot: IAuditableMultiBallot
    ) => Array<IDecodedVoteContest> | null
    checkIsBlank: (contest: IDecodedVoteContest) => boolean | null
    signHashableBallot: (
        ballotId: string,
        electionId: string,
        hashableBallot: IAuditableSingleBallot
    ) => ISignedContent | null
    signHashableMultiBallot: (
        ballotId: string,
        electionId: string,
        hashableBallot: IAuditableMultiBallot
    ) => ISignedContent | null
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
    signHashableBallot,
    signHashableMultiBallot,
})
