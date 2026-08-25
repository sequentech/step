// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Failures that belong to this application rather than to a ballot.
 *
 * `IInvalidPlaintextErrorType` moved into `@sequentech/ui-essentials` with the
 * warning list that reads it — a ballot's own complaints travel with the ballot.
 * What is left describes a *cast* that failed, which is a request this portal made
 * and nothing a shared component knows about.
 */

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

export {IInvalidPlaintextErrorType} from "@sequentech/ui-essentials"
