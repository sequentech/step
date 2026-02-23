// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    IAuditableBallot,
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    IAuditablePlaintextBallot,
    IBallotStyle,
    BallotSelection,
    ISignedContent,
    IDecodedVoteContest,
    IHashableBallot,
    EElectionEventContestEncryptionPolicy,
} from "@sequentech/ui-core"
import {IBallotService} from "./BallotService"
import {VotingPortalError, VotingPortalErrorType} from "./VotingPortalError"

// The interface all strategies must follow
export interface IBallotStrategy {
    encrypt(selection: BallotSelection, election: IBallotStyle): IAuditableBallot
    hash(ballot: IAuditableBallot): string
    toHashable(ballot: IAuditableBallot): IHashableBallot
    sign(ballotId: string, electionId: string, ballot: IAuditableBallot): ISignedContent | null
    decode(ballot: IAuditableBallot): Array<IDecodedVoteContest> | null
}

// --- Implementation: Single Contest ---
class SingleContestStrategy implements IBallotStrategy {
    constructor(private service: IBallotService) {}

    encrypt(selection: BallotSelection, election: IBallotStyle) {
        return this.service.encryptBallotSelection(selection, election)
    }

    hash(ballot: IAuditableBallot) {
        return this.service.hashBallot(ballot as IAuditableSingleBallot)
    }

    toHashable(ballot: IAuditableBallot) {
        return this.service.toHashableBallot(ballot as IAuditableSingleBallot)
    }

    sign(ballotId: string, electionId: string, ballot: IAuditableBallot) {
        return this.service.signHashableBallot(
            ballotId,
            electionId,
            ballot as IAuditableSingleBallot
        )
    }

    decode(ballot: IAuditableBallot) {
        return this.service.decodeAuditableBallot(ballot as IAuditableSingleBallot)
    }
}

// --- Implementation: Multiple Contests ---
class MultiContestStrategy implements IBallotStrategy {
    constructor(private service: IBallotService) {}

    encrypt(selection: BallotSelection, election: IBallotStyle) {
        return this.service.encryptMultiBallotSelection(selection, election)
    }

    hash(ballot: IAuditableBallot) {
        return this.service.hashMultiBallot(ballot as IAuditableMultiBallot)
    }

    toHashable(ballot: IAuditableBallot) {
        return this.service.toHashableMultiBallot(ballot as IAuditableMultiBallot)
    }

    sign(ballotId: string, electionId: string, ballot: IAuditableBallot) {
        return this.service.signHashableMultiBallot(
            ballotId,
            electionId,
            ballot as IAuditableMultiBallot
        )
    }

    decode(ballot: IAuditableBallot) {
        return this.service.decodeAuditableMultiBallot(ballot as IAuditableMultiBallot)
    }
}

// --- Implementation: Plaintext ---
class PlaintextStrategy implements IBallotStrategy {
    constructor(private service: IBallotService) {}

    encrypt(selection: BallotSelection, election: IBallotStyle) {
        return this.service.encodePlaintextBallotSelection(selection, election)
    }

    hash(ballot: IAuditableBallot) {
        return this.service.hashPlaintextBallot(ballot as IAuditablePlaintextBallot)
    }

    toHashable(ballot: IAuditableBallot) {
        return this.service.toHashablePlaintextBallot(ballot as IAuditablePlaintextBallot)
    }

    sign(ballotId: string, electionId: string, ballot: IAuditableBallot) {
        return this.service.signHashablePlaintextBallot(
            ballotId,
            electionId,
            ballot as IAuditablePlaintextBallot
        )
    }

    decode(ballot: IAuditableBallot) {
        return this.service.decodeAuditablePlaintextBallot(ballot as IAuditablePlaintextBallot)
    }
}

// --- Factory ---
export const getBallotStrategy = (
    policy: EElectionEventContestEncryptionPolicy | undefined,
    service: IBallotService
): IBallotStrategy => {
    switch (policy) {
        case EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS:
            return new MultiContestStrategy(service)
        case EElectionEventContestEncryptionPolicy.PLAINTEXT:
            return new PlaintextStrategy(service)
        case EElectionEventContestEncryptionPolicy.SINGLE_CONTEST:
            return new SingleContestStrategy(service)
        default:
            throw new VotingPortalError(VotingPortalErrorType.UNABLE_TO_ENCRYPT_BALLOT)
    }
}
