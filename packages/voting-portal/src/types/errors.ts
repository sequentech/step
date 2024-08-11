import { WasmCastBallotsErrorType } from "../services/VotingPortalError"

export enum EBallotError {
    PARSE_ERROR,
    DESERIALIZE_AUDITABLE_ERROR,
    DESERIALIZE_HASHABLE_ERROR,
    CONVERT_ERROR,
    SERIALIZE_ERROR,
}

export interface IBallotError {
    error_type: WasmCastBallotsErrorType
    error_msg: string
}
