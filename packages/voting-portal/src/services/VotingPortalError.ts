// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum VotingPortalErrorType {
    NO_ELECTION_EVENT = "NO_ELECTION_EVENT",
    INTERNAL_ERROR = "INTERNAL_ERROR",
    UNABLE_TO_FETCH_DATA = "UNABLE_TO_FETCH_DATA",
    UNABLE_TO_ENCRYPT_BALLOT = "UNABLE_TO_ENCRYPT_BALLOT",
    UNABLE_TO_CAST_BALLOT = "UNABLE_TO_CAST_BALLOT",
    NO_BALLOT_STYLE = "NO_BALLOT_STYLE",
    INCONSISTENT_HASH = "INCONSISTENT_HASH",
}

export enum ElectionScreenErrorType {
    FETCH_DATA = "unableToFetchData",
    NO_AREA = "noVotingArea",
    NETWORK = "networkError",
    NO_ELECTION_EVENT = "noElectionEvent",
    NO_CONTESTS = "noContests",
    WAITING_FOR_DATA = "waitingForData"
}

export enum ElectionScreenMsgType {
    NOT_PUBLISHED = "electionEventNotPublished",
    NO_ELECTIONS = "noElections",
}

export class VotingPortalError extends Error {
    type: VotingPortalErrorType

    constructor(type: VotingPortalErrorType) {
        super(type)
        this.name = "VotingPortalError"
        this.type = type
    }
}
