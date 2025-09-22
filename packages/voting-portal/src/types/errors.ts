// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {WasmCastBallotsErrorType} from "../services/VotingPortalError"

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
