// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IAuditableBallot, IAuditableMultiBallot, IAuditableSingleBallot} from "@sequentech/ui-core"
import {IBallotService} from "./BallotService"

export enum EBallotEncoding {
    SINGLE_CONTEST = "SINGLE_CONTEST",
    MULTI_CONTEST = "MULTI_CONTEST",
}

/**
 * Re-encrypts the plaintext and randomness carried by the auditable ballot and
 * compares the result with the ciphertext it carries. Returns true only when
 * they match; a mismatch and a ballot that cannot be checked both return false,
 * so the verifier fails closed.
 */
export const isAuditableBallotCiphertextConsistent = (
    ballotService: Pick<
        IBallotService,
        "verifyAuditableBallotCiphertext" | "verifyAuditableMultiBallotCiphertext"
    >,
    auditableBallot: IAuditableBallot,
    encoding: EBallotEncoding
): boolean => {
    try {
        return encoding === EBallotEncoding.MULTI_CONTEST
            ? ballotService.verifyAuditableMultiBallotCiphertext(
                  auditableBallot as IAuditableMultiBallot
              )
            : ballotService.verifyAuditableBallotCiphertext(
                  auditableBallot as IAuditableSingleBallot
              )
    } catch (error) {
        console.log(error)
        return false
    }
}
