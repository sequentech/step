// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * How the encoder classifies a complaint about a ballot.
 *
 * Split out of `voting-portal/src/types/errors.ts`, which also held `IBallotError`
 * — a cast-ballot failure, typed by `WasmCastBallotsErrorType` from the portal's
 * own error service. That is an application concern: it describes a request that
 * failed, not a ballot that is wrong. It stayed behind, and only this enum came,
 * because it is what the warning list reads to tell an explicit refusal from an
 * implicit one.
 */

export enum IInvalidPlaintextErrorType {
    /** The voter chose something the rules refuse — a marked-invalid option. */
    Explicit = "Explicit",
    /** The rules refuse the shape of the ballot — too few, too many, blank. */
    Implicit = "Implicit",
    EncodingError = "EncodingError",
}
