// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IAuditableBallot} from "@sequentech/ui-core"
import {
    EBallotCiphertextCheck,
    EBallotEncoding,
    checkAuditableBallotCiphertext,
} from "./ballotCiphertextVerification"

const auditableBallot = {ballot_hash: "hash"} as IAuditableBallot

const service = (single: () => boolean, multi: () => boolean) => ({
    verifyAuditableBallotCiphertext: jest.fn(single),
    verifyAuditableMultiBallotCiphertext: jest.fn(multi),
})

describe("checkAuditableBallotCiphertext", () => {
    it("accepts a single-contest ballot whose ciphertexts are reproduced", () => {
        const ballotService = service(
            () => true,
            () => false
        )

        expect(
            checkAuditableBallotCiphertext(
                ballotService,
                auditableBallot,
                EBallotEncoding.SINGLE_CONTEST
            )
        ).toBe(EBallotCiphertextCheck.VERIFIED)
        expect(ballotService.verifyAuditableBallotCiphertext).toHaveBeenCalledWith(auditableBallot)
        expect(ballotService.verifyAuditableMultiBallotCiphertext).not.toHaveBeenCalled()
    })

    it("accepts a multi-contest ballot whose ciphertext is reproduced", () => {
        const ballotService = service(
            () => false,
            () => true
        )

        expect(
            checkAuditableBallotCiphertext(
                ballotService,
                auditableBallot,
                EBallotEncoding.MULTI_CONTEST
            )
        ).toBe(EBallotCiphertextCheck.VERIFIED)
        expect(ballotService.verifyAuditableMultiBallotCiphertext).toHaveBeenCalledWith(
            auditableBallot
        )
        expect(ballotService.verifyAuditableBallotCiphertext).not.toHaveBeenCalled()
    })

    it.each([EBallotEncoding.SINGLE_CONTEST, EBallotEncoding.MULTI_CONTEST])(
        "reports a %s ballot whose ciphertext does not match as a mismatch",
        (encoding) => {
            const ballotService = service(
                () => false,
                () => false
            )

            expect(checkAuditableBallotCiphertext(ballotService, auditableBallot, encoding)).toBe(
                EBallotCiphertextCheck.MISMATCH
            )
        }
    )

    it.each([EBallotEncoding.SINGLE_CONTEST, EBallotEncoding.MULTI_CONTEST])(
        "reports a %s ballot that cannot be checked as not verifiable",
        (encoding) => {
            const fail = () => {
                throw new Error("Error checking the ballot")
            }
            const ballotService = service(fail, fail)

            expect(checkAuditableBallotCiphertext(ballotService, auditableBallot, encoding)).toBe(
                EBallotCiphertextCheck.NOT_VERIFIABLE
            )
        }
    )
})
