export enum VotingPortalErrorType {
    NoElectionEvent = "NoElectionEvent",
    InternalError = "InternalError",
    UnableToFetchData = "UnableToFetchData",
    UnableToEncryptBallot = "UnableToEncryptBallot",
    UnableToCastBallot = "UnableToCastBallot",
    NoBallotStyle = "NoBallotStyle",
}

export class VotingPortalError extends Error {
    type: VotingPortalErrorType

    constructor(type: VotingPortalErrorType) {
        super(type)
        this.name = "VotingPortalError"
        this.type = type
    }
}
