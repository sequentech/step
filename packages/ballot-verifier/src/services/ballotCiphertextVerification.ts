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
 * The outcome of re-encrypting the plaintext and randomness carried by an
 * auditable ballot and comparing the result with the ciphertext it carries.
 * A ballot that could not be checked at all is kept apart from one whose
 * ciphertext does not match, because only the latter says anything about the
 * ballot itself.
 */
export enum EBallotCiphertextCheck {
    VERIFIED = "VERIFIED",
    MISMATCH = "MISMATCH",
    NOT_VERIFIABLE = "NOT_VERIFIABLE",
}

/**
 * Re-encrypts the plaintext and randomness carried by the auditable ballot and
 * compares the result with the ciphertext it carries. Only VERIFIED lets the
 * ballot through, so the caller fails closed on the other two outcomes.
 */
export const checkAuditableBallotCiphertext = (
    ballotService: Pick<
        IBallotService,
        "verifyAuditableBallotCiphertext" | "verifyAuditableMultiBallotCiphertext"
    >,
    auditableBallot: IAuditableBallot,
    encoding: EBallotEncoding
): EBallotCiphertextCheck => {
    try {
        const isConsistent =
            encoding === EBallotEncoding.MULTI_CONTEST
                ? ballotService.verifyAuditableMultiBallotCiphertext(
                      auditableBallot as IAuditableMultiBallot
                  )
                : ballotService.verifyAuditableBallotCiphertext(
                      auditableBallot as IAuditableSingleBallot
                  )
        return isConsistent ? EBallotCiphertextCheck.VERIFIED : EBallotCiphertextCheck.MISMATCH
    } catch (error) {
        // The ballot could not be checked at all: a missing or malformed public
        // key, contests that do not match the ballot style, or a ballot read
        // with the wrong codec. None of that is evidence of a forged
        // ciphertext, so it must not be reported as one.
        console.log(error)
        return EBallotCiphertextCheck.NOT_VERIFIABLE
    }
}
