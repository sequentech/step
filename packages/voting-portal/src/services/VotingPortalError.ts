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
    PUBLICATION_NOT_FOUND = "publicationNotFound",
    NO_AREA = "noVotingArea",
    NETWORK = "networkError",
    NO_ELECTION_EVENT = "noElectionEvent",
    OBTAINING_ELECTION = "obtainingElectionFromID",
    BALLOT_STYLES_EML = "ballotStylesEmlError",
    UNKNOWN_ERROR = "unknownError",
    NO_AREA_CONTESTS = "noAreaContests",
}

export enum ElectionScreenMsgType {
    NOT_PUBLISHED = "electionEventNotPublished",
    NO_ELECTIONS = "noElections",
}

export enum CastBallotsErrorType {
    NETWORK_ERROR = "NETWORK_ERROR",
    UNABLE_TO_FETCH_DATA = "UNABLE_TO_FETCH_DATA",
    LOAD_ELECTION_EVENT = "LOAD_ELECTION_EVENT",
    CAST_VOTE = "CAST_VOTE",
    NO_BALLOT_SELECTION = "NO_BALLOT_SELECTION",
    NO_BALLOT_STYLE = "NO_BALLOT_STYLE",
    NO_AUDITABLE_BALLOT = "NO_AUDITABLE_BALLOT",
    INCONSISTENT_HASH = "INCONSISTENT_HASH",
    ELECTION_EVENT_NOT_OPEN = "ELECTION_EVENT_NOT_OPEN",
    UNKNOWN_ERROR = "UNKNOWN_ERROR",
}

export enum WasmCastBallotsErrorType {
    PARSE_ERROR = "PARSE_ERROR",
    DESERIALIZE_AUDITABLE_ERROR = "DESERIALIZE_AUDITABLE_ERROR",
    DESERIALIZE_HASHABLE_ERROR = "DESERIALIZE_HASHABLE_ERROR",
    CONVERT_ERROR = "CONVERT_ERROR",
    SERIALIZE_ERROR = "SERIALIZE_ERROR",
}

export class VotingPortalError extends Error {
    type: VotingPortalErrorType

    constructor(type: VotingPortalErrorType) {
        super(type)
        this.name = "VotingPortalError"
        this.type = type
    }
}
